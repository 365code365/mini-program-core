package dev.minirender

import android.media.MediaCodec
import android.media.MediaFormat
import android.util.Log
import org.json.JSONObject
import java.io.BufferedInputStream
import java.io.BufferedOutputStream
import java.io.ByteArrayOutputStream
import java.io.DataInputStream
import java.io.EOFException
import java.io.InputStream
import java.net.InetSocketAddress
import java.net.Socket
import java.net.URI
import java.net.UnknownHostException
import java.nio.ByteBuffer
import java.util.concurrent.LinkedBlockingDeque
import java.util.concurrent.TimeUnit

/**
 * `<live-pusher>` 的推流：把 [AvEncoder] 编出来的 H.264 / AAC 包成 FLV tag，经 RTMP 发给 `url`（官方：「目前仅支持 rtmp 格式」）。
 *
 * 只做推流端要的那一截协议：简单握手（C0 C1 / S0 S1 S2 / C2）→ `connect` → `releaseStream` / `FCPublish` / `createStream`
 * → `publish(live)` → `@setDataFrame onMetaData` → 两个序列头 → 音视频 tag。块大小发 4096；收方向只认
 * SetChunkSize / WindowAckSize / UserControl ping（回 pong）/ 命令回包（`_result` / `_error` / `onStatus`），其余读完丢掉。
 * 每条消息都用 fmt 0 的块头（12 字节，带绝对时间戳）—— 省掉增量头的状态，多出来的几个字节对推流码率无所谓。
 *
 * 状态码按官方 live-pusher 表：1001 连上服务器并握手、1002 开始推流、1009 发出第一帧画面；3001 DNS 失败、3002 连不上、
 * 3003 握手失败、3004 服务器断开 / 拒绝、3005 读写失败；断线后 1102 自动重连（最多 [RETRIES] 次），都不行 -1307。
 * 网络堵了（待发的包超过 [MAX_QUEUE]）丢掉没发的画面直到下一个关键帧，并让编码器马上出一个（[Listener.onCongested]）。
 */
internal class RtmpPublisher(private val url: String, private val listener: Listener) : AvEncoder.Sink {
    interface Listener {
        fun onState(code: Int, message: String)
        fun onNetStatus(info: JSONObject)
        /** 断线重连之后 / 丢过帧：要一个关键帧 */
        fun onCongested()
    }

    private class Packet(val video: Boolean, val key: Boolean, val ts: Int, val body: ByteArray)

    private val queue = LinkedBlockingDeque<Packet>()
    @Volatile private var running = true
    @Volatile private var publishing = false
    private var thread: Thread? = null
    @Volatile private var videoHeader: ByteArray? = null
    @Volatile private var audioHeader: ByteArray? = null
    private var basePts = -1L
    @Volatile private var waitKey = true
    @Volatile var videoWidth = 0
    @Volatile var videoHeight = 0
    @Volatile var gopSeconds = 2
    @Volatile var fps = 15

    // 每秒的统计
    private var statAt = 0L
    private var vBytes = 0L
    private var aBytes = 0L
    private var vFrames = 0
    private var firstVideoSent = false

    fun start() {
        thread = Thread({ run() }, "mini-rtmp").apply { start() }
    }

    /** 停：发送线程发完 FCUnpublish / deleteStream 再关连接（收端据此把文件收好尾）；1 秒没关掉就直接断 */
    fun stop() {
        running = false
        queue.clear()
        thread?.interrupt()
        val s = socket ?: return
        Thread {
            try { Thread.sleep(1000) } catch (_: InterruptedException) {}
            try { s.close() } catch (_: Exception) {}
        }.start()
    }

    // ───────── 编码器那一侧（编码线程上调） ─────────

    override fun onFormat(video: Boolean, format: MediaFormat) {
        if (video) {
            val sps = stripStartCode(format.getByteBuffer("csd-0") ?: return)
            val pps = stripStartCode(format.getByteBuffer("csd-1") ?: return)
            if (sps.size < 4) return
            val rec = ByteArrayOutputStream()
            rec.write(byteArrayOf(0x17, 0x00, 0, 0, 0))
            rec.write(byteArrayOf(1, sps[1], sps[2], sps[3], 0xFF.toByte(), 0xE1.toByte()))
            rec.write(byteArrayOf((sps.size shr 8).toByte(), sps.size.toByte()))
            rec.write(sps)
            rec.write(byteArrayOf(1, (pps.size shr 8).toByte(), pps.size.toByte()))
            rec.write(pps)
            videoHeader = rec.toByteArray()
        } else {
            val asc = format.getByteBuffer("csd-0") ?: return
            val b = ByteArray(asc.remaining())
            asc.duplicate().get(b)
            audioHeader = byteArrayOf(0xAF.toByte(), 0x00) + b
        }
    }

    override fun onSample(video: Boolean, buf: ByteBuffer, info: MediaCodec.BufferInfo) {
        if (!running) return
        if (basePts < 0) basePts = info.presentationTimeUs
        val ts = ((info.presentationTimeUs - basePts) / 1000).coerceAtLeast(0).toInt()
        val raw = ByteArray(buf.remaining())
        buf.get(raw)
        if (!publishing) return
        if (video) {
            val key = info.flags and MediaCodec.BUFFER_FLAG_KEY_FRAME != 0
            if (waitKey && !key) return
            waitKey = false
            val avcc = annexBToAvcc(raw)
            if (avcc.isEmpty()) return
            val body = ByteArray(5 + avcc.size)
            body[0] = if (key) 0x17 else 0x27
            body[1] = 0x01
            System.arraycopy(avcc, 0, body, 5, avcc.size)
            queue.offer(Packet(true, key, ts, body))
        } else {
            queue.offer(Packet(false, false, ts, byteArrayOf(0xAF.toByte(), 0x01) + raw))
        }
        if (queue.size > MAX_QUEUE) congested()
    }

    /** 堵了：没发的画面全丢（声音留着），等下一个关键帧 */
    private fun congested() {
        queue.removeIf { it.video }
        waitKey = true
        listener.onCongested()
    }

    // ───────── 发送线程 ─────────

    private var socket: Socket? = null
    private var out: BufferedOutputStream? = null
    private var inChunk = 128
    private var streamId = 1

    private fun run() {
        var attempt = 0
        while (running) {
            val code = try {
                session(attempt)
                0
            } catch (e: RtmpError) {
                e.code
            } catch (e: InterruptedException) {
                0
            } catch (e: Exception) {
                if (running) Log.w(TAG, "📡 推流断了：$e")
                3005
            }
            publishing = false
            try { socket?.close() } catch (_: Exception) {}
            socket = null
            if (!running) return
            if (code != 0 && attempt == 0) listener.onState(code, MESSAGES[code] ?: "推流失败")
            attempt++
            if (attempt > RETRIES) {
                listener.onState(-1307, MESSAGES[-1307]!!)
                running = false
                return
            }
            listener.onState(1102, MESSAGES[1102]!!)
            try { Thread.sleep(1000L * attempt) } catch (_: InterruptedException) { return }
        }
    }

    private class RtmpError(val code: Int, msg: String) : Exception(msg)

    private fun session(attempt: Int) {
        val u = try { URI(url) } catch (e: Exception) { throw RtmpError(3004, "推流地址不对：$url") }
        if (u.scheme?.lowercase() != "rtmp") throw RtmpError(3004, "只支持 rtmp 地址")
        val host = u.host ?: throw RtmpError(3001, "没有主机名")
        val port = if (u.port > 0) u.port else 1935
        val parts = (u.rawPath ?: "").trim('/').split('/', limit = 2)
        val app = parts.getOrElse(0) { "" }
        val stream = parts.getOrElse(1) { "" } + (u.rawQuery?.let { "?$it" } ?: "")
        if (app.isEmpty() || stream.isEmpty()) throw RtmpError(3004, "推流地址缺 app 或流名：$url")
        val tcUrl = "rtmp://$host${if (u.port > 0) ":$port" else ""}/$app"
        val s = Socket()
        try {
            s.connect(InetSocketAddress(host, port), 5000)
        } catch (e: UnknownHostException) {
            throw RtmpError(3001, "$e")
        } catch (e: Exception) {
            throw RtmpError(3002, "$e")
        }
        s.tcpNoDelay = true
        s.soTimeout = 8000
        socket = s
        val o = BufferedOutputStream(s.getOutputStream(), 64 * 1024)
        out = o
        val input = DataInputStream(BufferedInputStream(s.getInputStream()))
        try {
            handshake(o, input)
        } catch (e: Exception) {
            throw RtmpError(3003, "$e")
        }
        if (attempt == 0) listener.onState(1001, MESSAGES[1001]!!)
        inChunk = 128
        send(2, 1, 0, 0, byteArrayOf(0, 0, 0x10, 0)) // SetChunkSize 4096
        command(3, 0, "connect", 1.0, Amf.obj("app" to app, "type" to "nonprivate", "flashVer" to "FMLE/3.0 (compatible; FMSc/1.0)", "swfUrl" to tcUrl, "tcUrl" to tcUrl))
        expectResult(input, 1.0)
        command(3, 0, "releaseStream", 2.0, null, stream)
        command(3, 0, "FCPublish", 3.0, null, stream)
        command(3, 0, "createStream", 4.0, null)
        val created = expectResult(input, 4.0)
        streamId = (created.getOrNull(3) as? Double)?.toInt() ?: 1
        command(8, streamId, "publish", 5.0, null, stream, "live")
        while (true) {
            val r = readCommand(input) ?: continue
            if (r[0] == "onStatus") {
                val info = r.getOrNull(3) as? Map<*, *>
                val code = info?.get("code") as? String ?: ""
                if (code == "NetStream.Publish.Start") break
                throw RtmpError(3004, "服务器拒绝推流：$code ${info?.get("description") ?: ""}")
            }
            if (r[0] == "_error") throw RtmpError(3004, "服务器拒绝推流")
        }
        s.soTimeout = 0
        // 收方向另起一条线程读（回 ping、发现断线），发方向留在这里
        val reader = Thread({ drainIncoming(input) }, "mini-rtmp-in").apply { isDaemon = true; start() }
        sendMeta()
        videoHeader?.let { send(6, 9, streamId, 0, it) }
        audioHeader?.let { send(4, 8, streamId, 0, it) }
        flush()
        waitKey = true
        publishing = true
        listener.onCongested()
        listener.onState(1002, MESSAGES[1002]!!)
        statAt = System.currentTimeMillis()
        try {
            while (running) {
                val p = queue.poll(200, TimeUnit.MILLISECONDS)
                if (!reader.isAlive) throw RtmpError(3004, "服务器断开了")
                if (p != null) {
                    send(if (p.video) 6 else 4, if (p.video) 9 else 8, streamId, p.ts, p.body)
                    if (p.video) {
                        vBytes += p.body.size
                        vFrames++
                        if (!firstVideoSent) {
                            firstVideoSent = true
                            listener.onState(1009, MESSAGES[1009]!!)
                        }
                    } else {
                        aBytes += p.body.size
                    }
                    if (queue.isEmpty()) flush()
                }
                tickStats()
            }
            flush()
            // 正常停：告诉服务器这一路不推了
            command(3, 0, "FCUnpublish", 6.0, null, stream)
            command(3, 0, "deleteStream", 7.0, null, streamId.toDouble())
        } catch (e: InterruptedException) {
            try {
                command(3, 0, "FCUnpublish", 6.0, null, stream)
                command(3, 0, "deleteStream", 7.0, null, streamId.toDouble())
            } catch (_: Exception) {
            }
        } finally {
            publishing = false
        }
    }

    private fun tickStats() {
        val now = System.currentTimeMillis()
        val dt = now - statAt
        if (dt < 1000) return
        val vk = vBytes * 8 / dt.toDouble()
        val ak = aBytes * 8 / dt.toDouble()
        val fpsNow = vFrames * 1000.0 / dt
        val vq = queue.count { it.video }
        val aq = queue.size - vq
        val level = when {
            queue.size < 10 -> 1
            queue.size < 40 -> 2
            queue.size < 120 -> 3
            else -> 5
        }
        listener.onNetStatus(JSONObject()
            .put("videoBitrate", vk.toInt()).put("audioBitrate", ak.toInt()).put("videoFPS", Math.round(fpsNow))
            .put("videoGOP", gopSeconds).put("netSpeed", (vk + ak).toInt()).put("netJitter", 0).put("netQualityLevel", level)
            .put("videoWidth", videoWidth).put("videoHeight", videoHeight).put("videoCache", vq).put("audioCache", aq))
        statAt = now
        vBytes = 0
        aBytes = 0
        vFrames = 0
    }

    private fun handshake(o: BufferedOutputStream, input: DataInputStream) {
        val c1 = ByteArray(1536)
        java.util.Random().nextBytes(c1)
        for (i in 0 until 8) c1[i] = 0 // time 0 + zero
        o.write(3)
        o.write(c1)
        flush()
        val s0 = input.readUnsignedByte()
        if (s0 != 3) throw RtmpError(3003, "服务器的 RTMP 版本 $s0")
        val s1 = ByteArray(1536)
        input.readFully(s1)
        val s2 = ByteArray(1536)
        input.readFully(s2)
        o.write(s1) // C2 = S1
        flush()
    }

    // ───────── 块的收发 ─────────

    private val chunkOut = 4096

    /** 收发两条线程都会写（回 pong）：写与刷都在同一把锁里 */
    @Synchronized
    private fun flush() {
        out?.flush()
    }

    @Synchronized
    private fun send(csid: Int, type: Int, msgStream: Int, ts: Int, payload: ByteArray) {
        val o = out ?: return
        val ext = ts >= 0xFFFFFF
        val t3 = if (ext) 0xFFFFFF else ts
        o.write(csid and 0x3F)
        o.write(byteArrayOf((t3 shr 16).toByte(), (t3 shr 8).toByte(), t3.toByte()))
        o.write(byteArrayOf((payload.size shr 16).toByte(), (payload.size shr 8).toByte(), payload.size.toByte()))
        o.write(type)
        o.write(byteArrayOf(msgStream.toByte(), (msgStream shr 8).toByte(), (msgStream shr 16).toByte(), (msgStream shr 24).toByte()))
        if (ext) o.write(be32(ts))
        var off = 0
        while (off < payload.size) {
            if (off > 0) {
                o.write(0xC0 or (csid and 0x3F))
                if (ext) o.write(be32(ts))
            }
            val n = minOf(chunkOut, payload.size - off)
            o.write(payload, off, n)
            off += n
        }
    }

    private fun be32(v: Int) = byteArrayOf((v shr 24).toByte(), (v shr 16).toByte(), (v shr 8).toByte(), v.toByte())

    private fun command(csid: Int, msgStream: Int, name: String, txn: Double, obj: Any?, vararg args: Any?) {
        val b = ByteArrayOutputStream()
        Amf.write(b, name)
        Amf.write(b, txn)
        Amf.write(b, obj)
        for (a in args) Amf.write(b, a)
        send(csid, 20, msgStream, 0, b.toByteArray())
        flush()
    }

    private fun sendMeta() {
        val b = ByteArrayOutputStream()
        Amf.write(b, "@setDataFrame")
        Amf.write(b, "onMetaData")
        Amf.writeEcma(b, linkedMapOf(
            "width" to videoWidth.toDouble(), "height" to videoHeight.toDouble(), "videocodecid" to 7.0, "audiocodecid" to 10.0,
            "framerate" to fps.toDouble(), "audiochannels" to 1.0, "stereo" to false,
        ))
        send(5, 18, streamId, 0, b.toByteArray())
    }

    private class Msg(val type: Int, val payload: ByteArray)

    private class ChunkState(var ts: Int = 0, var len: Int = 0, var type: Int = 0, var stream: Int = 0, var buf: ByteArrayOutputStream? = null)

    private val chunks = HashMap<Int, ChunkState>()

    /** 读一条完整的消息（块拼起来）；控制消息就地处理 */
    private fun readMessage(input: DataInputStream): Msg {
        while (true) {
            val b0 = input.readUnsignedByte()
            val fmt = b0 shr 6
            var csid = b0 and 0x3F
            if (csid == 0) csid = 64 + input.readUnsignedByte()
            else if (csid == 1) csid = 64 + input.readUnsignedByte() + input.readUnsignedByte() * 256
            val st = chunks.getOrPut(csid) { ChunkState() }
            var ts = st.ts
            if (fmt <= 2) ts = read24(input)
            if (fmt <= 1) {
                st.len = read24(input)
                st.type = input.readUnsignedByte()
            }
            if (fmt == 0) st.stream = Integer.reverseBytes(input.readInt())
            if (ts == 0xFFFFFF) ts = input.readInt()
            st.ts = ts
            val acc = st.buf ?: ByteArrayOutputStream().also { st.buf = it }
            val n = minOf(inChunk, st.len - acc.size())
            val part = ByteArray(n)
            input.readFully(part)
            acc.write(part)
            if (acc.size() < st.len) continue
            st.buf = null
            val m = Msg(st.type, acc.toByteArray())
            when (m.type) {
                1 -> if (m.payload.size >= 4) inChunk = ByteBuffer.wrap(m.payload).int and 0x7FFFFFFF
                4 -> if (m.payload.size >= 6 && ByteBuffer.wrap(m.payload).short.toInt() == 6) {
                    // ping → pong（同一个时间戳）
                    send(2, 4, 0, 0, byteArrayOf(0, 7) + m.payload.copyOfRange(2, 6))
                    flush()
                }
            }
            return m
        }
    }

    private fun read24(input: InputStream): Int {
        val a = input.read()
        val b = input.read()
        val c = input.read()
        if (a or b or c < 0) throw EOFException()
        return (a shl 16) or (b shl 8) or c
    }

    private fun readCommand(input: DataInputStream): List<Any?>? {
        val m = readMessage(input)
        if (m.type != 20) return null
        return Amf.readAll(m.payload)
    }

    private fun expectResult(input: DataInputStream, txn: Double): List<Any?> {
        while (true) {
            val r = readCommand(input) ?: continue
            if (r.getOrNull(1) != txn) continue
            if (r[0] == "_result") return r
            val info = r.getOrNull(3) as? Map<*, *>
            throw RtmpError(3004, "服务器拒绝：${r[0]} ${info?.get("code") ?: ""} ${info?.get("description") ?: ""}")
        }
    }

    private fun drainIncoming(input: DataInputStream) {
        try {
            while (running) {
                val m = readMessage(input)
                if (m.type == 20) {
                    val r = Amf.readAll(m.payload)
                    val info = r.getOrNull(3) as? Map<*, *>
                    val code = info?.get("code") as? String ?: ""
                    if (r[0] == "onStatus" && (code.contains("Failed") || code.contains("BadName") || code.contains("Unpublish"))) return
                }
            }
        } catch (_: Exception) {
        }
    }

    internal companion object {
        const val TAG = "mini-render"
        const val RETRIES = 3
        const val MAX_QUEUE = 240
        val MESSAGES = mapOf(
            1001 to "已经连接推流服务器", 1002 to "已经与服务器握手完毕,开始推流", 1009 to "推流：发送视频首帧",
            1102 to "网络断连, 已启动自动重连", 3001 to "RTMP -DNS解析失败", 3002 to "RTMP服务器连接失败",
            3003 to "RTMP服务器握手失败", 3004 to "RTMP服务器主动断开，请检查推流地址的合法性或防盗链有效期", 3005 to "RTMP 读/写失败",
            -1307 to "网络断连，且经多次重连抢救无效，更多重试请自行重启推流",
        )

        /** `00 00 00 01` / `00 00 01` 开头的 Annex-B → 去掉开头的起始码（csd 里的 SPS / PPS） */
        fun stripStartCode(b: ByteBuffer): ByteArray {
            val a = ByteArray(b.remaining())
            b.duplicate().get(a)
            var i = 0
            while (i < a.size && a[i].toInt() == 0) i++
            if (i < a.size && a[i].toInt() == 1 && i >= 2) i++ else i = 0
            return a.copyOfRange(i, a.size)
        }

        /** Annex-B 的一帧 → AVCC（每个 NAL 前 4 字节长度）。SPS / PPS / AUD 不带（序列头里已经有了） */
        fun annexBToAvcc(a: ByteArray): ByteArray {
            val starts = ArrayList<Pair<Int, Int>>() // (起始码位置, NAL 开头)
            var i = 0
            while (i + 3 <= a.size) {
                if (a[i].toInt() == 0 && a[i + 1].toInt() == 0) {
                    if (a[i + 2].toInt() == 1) { starts += i to i + 3; i += 3; continue }
                    if (i + 4 <= a.size && a[i + 2].toInt() == 0 && a[i + 3].toInt() == 1) { starts += i to i + 4; i += 4; continue }
                }
                i++
            }
            if (starts.isEmpty()) starts += 0 to 0
            val outB = ByteArrayOutputStream(a.size + 16)
            for ((k, s) in starts.withIndex()) {
                val end = if (k + 1 < starts.size) starts[k + 1].first else a.size
                val len = end - s.second
                if (len <= 0) continue
                val type = a[s.second].toInt() and 0x1F
                if (type == 7 || type == 8 || type == 9) continue
                outB.write(byteArrayOf((len shr 24).toByte(), (len shr 16).toByte(), (len shr 8).toByte(), len.toByte()))
                outB.write(a, s.second, len)
            }
            return outB.toByteArray()
        }
    }
}

/** AMF0：推流端用得到的那几种值 */
internal object Amf {
    fun obj(vararg kv: Pair<String, Any?>): Map<String, Any?> = linkedMapOf(*kv)

    fun write(o: ByteArrayOutputStream, v: Any?) {
        when (v) {
            null -> o.write(5)
            is Double -> { o.write(0); o.write(ByteBuffer.allocate(8).putDouble(v).array()) }
            is Int -> write(o, v.toDouble())
            is Boolean -> { o.write(1); o.write(if (v) 1 else 0) }
            is String -> { o.write(2); str(o, v) }
            is Map<*, *> -> {
                o.write(3)
                for ((k, x) in v) { str(o, k.toString()); write(o, x) }
                o.write(byteArrayOf(0, 0, 9))
            }
            else -> write(o, v.toString())
        }
    }

    fun writeEcma(o: ByteArrayOutputStream, m: Map<String, Any?>) {
        o.write(8)
        o.write(ByteBuffer.allocate(4).putInt(m.size).array())
        for ((k, x) in m) { str(o, k); write(o, x) }
        o.write(byteArrayOf(0, 0, 9))
    }

    private fun str(o: ByteArrayOutputStream, s: String) {
        val b = s.toByteArray(Charsets.UTF_8)
        o.write(byteArrayOf((b.size shr 8).toByte(), b.size.toByte()))
        o.write(b)
    }

    fun readAll(p: ByteArray): List<Any?> {
        val b = ByteBuffer.wrap(p)
        val out = ArrayList<Any?>()
        try {
            while (b.hasRemaining()) out += read(b)
        } catch (_: Exception) {
        }
        return out
    }

    private fun readStr(b: ByteBuffer): String {
        val n = b.short.toInt() and 0xFFFF
        val a = ByteArray(n)
        b.get(a)
        return String(a, Charsets.UTF_8)
    }

    private fun read(b: ByteBuffer): Any? = when (val t = b.get().toInt()) {
        0 -> b.double
        1 -> b.get().toInt() != 0
        2 -> readStr(b)
        3, 8 -> {
            if (t == 8) b.int
            val m = LinkedHashMap<String, Any?>()
            while (true) {
                val k = readStr(b)
                if (k.isEmpty() && b.get(b.position()).toInt() == 9) { b.get(); break }
                m[k] = read(b)
            }
            m
        }
        5, 6 -> null
        10 -> { val n = b.int; List(n) { read(b) } }
        else -> throw IllegalStateException("AMF0 类型 $t")
    }
}
