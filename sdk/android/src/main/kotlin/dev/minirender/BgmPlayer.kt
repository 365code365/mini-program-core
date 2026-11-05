package dev.minirender

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.media.MediaCodec
import android.media.MediaExtractor
import android.media.MediaFormat
import android.util.Log
import kotlin.math.roundToInt

/**
 * live-pusher 的背景音（LivePusherContext.playBGM / pauseBGM / resumeBGM / stopBGM / setBGMVolume）：
 * 一条线程（`mini-bgm`）`MediaExtractor` + `MediaCodec` 解成 PCM，同一份 PCM 两处用：
 * - **本地放**（主播自己听得到）：`AudioTrack` 阻塞写，写的快慢就是播放的快慢，解码不会跑在前面；
 * - **混进推流**：缩成单声道、按推流的采样率线性重采样，放进一个最多 1 秒的队列，[AvEncoder] 的麦克风线程每读一段麦克风
 *   就从这里取同样多的样本加上去（[mixInto]）。没有推流时队列满了丢最旧的，开推那一刻不会一下子灌进去一大段。
 *
 * 外放时麦克风会把喇叭里的背景音再录进去一遍（这里没有回声消除，腾讯云 SDK 有），戴耳机推流没有这个问题。
 * 事件：开始放 [Listener.onStart]、每 500ms [Listener.onProgress]（毫秒）、放完 [Listener.onComplete]、打不开 [Listener.onError]（errCode 10003）。
 */
internal class BgmPlayer(private val listener: Listener) {
    interface Listener {
        fun onStart()
        fun onProgress(progressMs: Long, durationMs: Long)
        fun onComplete()
        fun onError(msg: String)
    }

    @Volatile private var running = false
    @Volatile private var paused = false
    @Volatile var volume = 1f
        set(v) {
            field = v.coerceIn(0f, 1f)
            track?.setVolume(field)
        }
    /** 推流的采样率（[AvEncoder] 开起来之后设；0 = 没在推，不攒） */
    @Volatile var mixRate = 0
    private var thread: Thread? = null
    @Volatile private var track: AudioTrack? = null
    private val lock = Object()
    private val queue = ShortArray(48_000)
    private var head = 0
    private var size = 0

    val playing: Boolean get() = running

    /** 换一首放（[source] 是本地绝对路径或 http(s) 地址）。正在放的先停 */
    fun play(source: String) {
        stop()
        running = true
        paused = false
        thread = Thread({ run(source) }, "mini-bgm").apply { start() }
    }

    fun pause() {
        paused = true
        track?.pause()
    }

    fun resume() {
        paused = false
        track?.play()
        synchronized(lock) { lock.notifyAll() }
    }

    fun stop() {
        running = false
        synchronized(lock) { lock.notifyAll() }
        // 暂停着的时候解码线程可能卡在 AudioTrack 的阻塞写里：停掉 track 它才返回
        try {
            track?.stop()
        } catch (_: Exception) {
        }
        thread?.join(800)
        thread = null
        synchronized(lock) {
            head = 0
            size = 0
        }
    }

    /** 麦克风那一段 [pcm]（前 [n] 个，单声道、推流采样率）加上同样多的背景音（麦克风线程上调） */
    fun mixInto(pcm: ShortArray, n: Int) {
        if (!running || paused) return
        val g = volume
        synchronized(lock) {
            val m = minOf(n, size)
            for (k in 0 until m) {
                val v = pcm[k] + (queue[(head + k) % queue.size] * g).toInt()
                pcm[k] = v.coerceIn(-32768, 32767).toShort()
            }
            head = (head + m) % queue.size
            size -= m
        }
    }

    private fun offer(mono: ShortArray, n: Int) {
        synchronized(lock) {
            for (k in 0 until n) {
                if (size == queue.size) {
                    head = (head + 1) % queue.size
                    size--
                }
                queue[(head + size) % queue.size] = mono[k]
                size++
            }
        }
    }

    private fun run(source: String) {
        val ex = MediaExtractor()
        var dec: MediaCodec? = null
        var at: AudioTrack? = null
        try {
            ex.setDataSource(source)
            val ti = (0 until ex.trackCount).firstOrNull { ex.getTrackFormat(it).getString(MediaFormat.KEY_MIME)?.startsWith("audio/") == true }
                ?: throw IllegalStateException("没有音轨")
            ex.selectTrack(ti)
            val f = ex.getTrackFormat(ti)
            val durUs = if (f.containsKey(MediaFormat.KEY_DURATION)) f.getLong(MediaFormat.KEY_DURATION) else 0L
            var rate = f.getInteger(MediaFormat.KEY_SAMPLE_RATE)
            var ch = f.getInteger(MediaFormat.KEY_CHANNEL_COUNT)
            val d = MediaCodec.createDecoderByType(f.getString(MediaFormat.KEY_MIME)!!)
            dec = d
            d.configure(f, null, null, 0)
            d.start()
            var tr = newTrack(rate, ch)
            at = tr
            track = tr
            tr.play()
            listener.onStart()
            val info = MediaCodec.BufferInfo()
            var inputDone = false
            var lastReport = 0L
            var pcm = ShortArray(0)
            var mono = ShortArray(0)
            var phase = 0.0
            while (running) {
                if (paused) {
                    synchronized(lock) { if (paused && running) lock.wait(200) }
                    continue
                }
                if (!inputDone) {
                    val ii = d.dequeueInputBuffer(5_000)
                    if (ii >= 0) {
                        val n = ex.readSampleData(d.getInputBuffer(ii)!!, 0)
                        if (n < 0) {
                            d.queueInputBuffer(ii, 0, 0, 0, MediaCodec.BUFFER_FLAG_END_OF_STREAM)
                            inputDone = true
                        } else {
                            d.queueInputBuffer(ii, 0, n, ex.sampleTime, 0)
                            ex.advance()
                        }
                    }
                }
                val oi = d.dequeueOutputBuffer(info, 5_000)
                if (oi == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                    val of = d.outputFormat
                    val r2 = of.getInteger(MediaFormat.KEY_SAMPLE_RATE)
                    val c2 = of.getInteger(MediaFormat.KEY_CHANNEL_COUNT)
                    if (r2 != rate || c2 != ch) {
                        rate = r2
                        ch = c2
                        tr.release()
                        tr = newTrack(rate, ch)
                        at = tr
                        track = tr
                        tr.play()
                    }
                    continue
                }
                if (oi < 0) continue
                val buf = d.getOutputBuffer(oi)
                val samples = info.size / 2
                if (buf != null && samples > 0) {
                    if (pcm.size < samples) pcm = ShortArray(samples)
                    buf.position(info.offset)
                    buf.order(java.nio.ByteOrder.nativeOrder()).asShortBuffer().get(pcm, 0, samples)
                    // 推着流：缩成单声道、重采样到推流的采样率，进混音队列
                    val target = mixRate
                    if (target > 0) {
                        val frames = samples / ch
                        val step = rate.toDouble() / target
                        val outN = ((frames - phase) / step).toInt().coerceAtLeast(0)
                        if (mono.size < outN) mono = ShortArray(outN)
                        for (k in 0 until outN) {
                            val pos = phase + k * step
                            val i0 = pos.toInt().coerceIn(0, frames - 1)
                            val i1 = (i0 + 1).coerceAtMost(frames - 1)
                            val t = pos - i0
                            var s0 = 0
                            var s1 = 0
                            for (c in 0 until ch) {
                                s0 += pcm[i0 * ch + c]
                                s1 += pcm[i1 * ch + c]
                            }
                            mono[k] = ((s0 * (1 - t) + s1 * t) / ch).roundToInt().coerceIn(-32768, 32767).toShort()
                        }
                        phase = phase + outN * step - frames
                        offer(mono, outN)
                    }
                    tr.write(pcm, 0, samples)
                }
                d.releaseOutputBuffer(oi, false)
                if (info.presentationTimeUs / 1000 - lastReport >= 500) {
                    lastReport = info.presentationTimeUs / 1000
                    listener.onProgress(lastReport, durUs / 1000)
                }
                if (info.flags and MediaCodec.BUFFER_FLAG_END_OF_STREAM != 0) {
                    listener.onProgress(durUs / 1000, durUs / 1000)
                    running = false
                    listener.onComplete()
                    break
                }
            }
        } catch (e: Exception) {
            Log.w("mini-render", "📡 背景音打不开：$e")
            running = false
            listener.onError("$e")
        } finally {
            track = null
            try {
                at?.stop()
            } catch (_: Exception) {
            }
            at?.release()
            try {
                dec?.stop()
            } catch (_: Exception) {
            }
            dec?.release()
            ex.release()
        }
    }

    private fun newTrack(rate: Int, ch: Int): AudioTrack {
        val mask = if (ch >= 2) AudioFormat.CHANNEL_OUT_STEREO else AudioFormat.CHANNEL_OUT_MONO
        val min = AudioTrack.getMinBufferSize(rate, mask, AudioFormat.ENCODING_PCM_16BIT)
        return AudioTrack.Builder()
            .setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).setContentType(AudioAttributes.CONTENT_TYPE_MUSIC).build())
            .setAudioFormat(AudioFormat.Builder().setSampleRate(rate).setChannelMask(mask).setEncoding(AudioFormat.ENCODING_PCM_16BIT).build())
            .setBufferSizeInBytes(maxOf(min, rate / 5 * 2 * maxOf(1, ch)))
            .build()
            .also { it.setVolume(volume) }
    }
}
