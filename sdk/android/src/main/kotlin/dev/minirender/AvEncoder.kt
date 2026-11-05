package dev.minirender

import android.annotation.SuppressLint
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.media.MediaMuxer
import android.media.MediaRecorder
import android.os.Bundle
import android.util.Log
import android.view.Surface
import java.io.File
import java.nio.ByteBuffer

/**
 * 录像（`CameraContext.startRecord`）与推流（`<live-pusher>`）共用的编码器：
 * - 视频：H.264 硬编，**Surface 输入**（[surface] 交给 [CameraPipe.attachEncoder]，相机线程每帧画进去）；
 * - 音频：麦克风 `AudioRecord` → AAC-LC（[audio] 为 null 时只有画面：没有录音权限、`enable-mic=false`）。
 *
 * 编码出来的东西交给 [Sink]：录像是 [Mp4Sink]（`MediaMuxer`），推流是 [RtmpPublisher]。
 * 两路各一条线程取输出（`mini-venc` / `mini-mic`），时间戳都按 `System.nanoTime()`（相机那边画进 Surface 时也按它打），音画同一个时钟。
 */
internal class AvEncoder(
    val width: Int,
    val height: Int,
    val fps: Int,
    private val bitrate: Int,
    private val gopSeconds: Int,
    private val audio: AudioCfg?,
    private val sink: Sink,
    private val onError: (video: Boolean, msg: String) -> Unit,
) {
    class AudioCfg(val sampleRate: Int, val bitrate: Int)

    interface Sink {
        fun onFormat(video: Boolean, format: MediaFormat)
        /** 一包编码数据（[buf] 只在这次调用里有效） */
        fun onSample(video: Boolean, buf: ByteBuffer, info: MediaCodec.BufferInfo)
    }

    private val venc: MediaCodec = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_VIDEO_AVC)
    val surface: Surface
    private var aenc: MediaCodec? = null
    private var mic: AudioRecord? = null
    @Volatile private var running = true
    private var vThread: Thread? = null
    private var aThread: Thread? = null
    @Volatile var micMuted = false
    @Volatile var micVolume = 1f
    /** live-pusher 的背景音：每段麦克风 PCM 混上同样多的背景音（静音的是麦克风，背景音照推） */
    @Volatile var bgm: BgmPlayer? = null
    val audioRate: Int get() = audio?.sampleRate ?: 0

    init {
        val f = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, width, height)
        f.setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
        f.setInteger(MediaFormat.KEY_BIT_RATE, bitrate)
        f.setInteger(MediaFormat.KEY_FRAME_RATE, fps)
        f.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, gopSeconds)
        venc.configure(f, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
        surface = venc.createInputSurface()
    }

    /** 开始编码。音频开不起来（麦克风被占 / 没权限）时只编画面，[onError] 报一次 */
    @SuppressLint("MissingPermission")
    fun start() {
        venc.start()
        vThread = Thread({ drainVideo() }, "mini-venc").apply { start() }
        val a = audio ?: return
        try {
            val min = AudioRecord.getMinBufferSize(a.sampleRate, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT)
            val rec = AudioRecord(MediaRecorder.AudioSource.MIC, a.sampleRate, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT, max(min, 4096) * 2)
            if (rec.state != AudioRecord.STATE_INITIALIZED) {
                rec.release()
                onError(false, "麦克风打不开")
                return
            }
            val af = MediaFormat.createAudioFormat(MediaFormat.MIMETYPE_AUDIO_AAC, a.sampleRate, 1)
            af.setInteger(MediaFormat.KEY_AAC_PROFILE, MediaCodecInfo.CodecProfileLevel.AACObjectLC)
            af.setInteger(MediaFormat.KEY_BIT_RATE, a.bitrate)
            af.setInteger(MediaFormat.KEY_MAX_INPUT_SIZE, 8192)
            val enc = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_AUDIO_AAC)
            enc.configure(af, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
            enc.start()
            rec.startRecording()
            mic = rec
            aenc = enc
            aThread = Thread({ pumpAudio(rec, enc) }, "mini-mic").apply { start() }
        } catch (e: Exception) {
            onError(false, "麦克风打不开：$e")
        }
    }

    private fun max(a: Int, b: Int) = if (a > b) a else b

    /** 立刻出一个关键帧（推流断线重连之后） */
    fun requestKeyFrame() {
        try {
            venc.setParameters(Bundle().apply { putInt(MediaCodec.PARAMETER_KEY_REQUEST_SYNC_FRAME, 0) })
        } catch (_: Exception) {
        }
    }

    /** 码率（推流时按网络调） */
    fun setBitrate(bps: Int) {
        try {
            venc.setParameters(Bundle().apply { putInt(MediaCodec.PARAMETER_KEY_VIDEO_BITRATE, bps) })
        } catch (_: Exception) {
        }
    }

    private fun drainVideo() {
        val info = MediaCodec.BufferInfo()
        while (true) {
            val i = try {
                venc.dequeueOutputBuffer(info, 10_000)
            } catch (e: Exception) {
                if (running) onError(true, "视频编码失败：$e")
                return
            }
            when {
                i == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED -> sink.onFormat(true, venc.outputFormat)
                i >= 0 -> {
                    val buf = venc.getOutputBuffer(i)
                    if (buf != null && info.size > 0 && info.flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG == 0) {
                        buf.position(info.offset)
                        buf.limit(info.offset + info.size)
                        sink.onSample(true, buf, info)
                    }
                    venc.releaseOutputBuffer(i, false)
                    if (info.flags and MediaCodec.BUFFER_FLAG_END_OF_STREAM != 0) return
                }
                !running -> return
            }
        }
    }

    private fun pumpAudio(rec: AudioRecord, enc: MediaCodec) {
        val pcm = ShortArray(1024)
        val info = MediaCodec.BufferInfo()
        while (running) {
            val n = rec.read(pcm, 0, pcm.size)
            if (n <= 0) continue
            val pts = System.nanoTime() / 1000
            if (micMuted) pcm.fill(0, 0, n) else if (micVolume != 1f) {
                for (k in 0 until n) pcm[k] = (pcm[k] * micVolume).toInt().coerceIn(-32768, 32767).toShort()
            }
            bgm?.mixInto(pcm, n)
            val ii = enc.dequeueInputBuffer(10_000)
            if (ii >= 0) {
                val b = enc.getInputBuffer(ii)!!
                b.clear()
                // PCM 16 位按本机字节序（小端）；ByteBuffer 默认大端，不改的话全是噪声
                b.order(java.nio.ByteOrder.nativeOrder()).asShortBuffer().put(pcm, 0, n)
                enc.queueInputBuffer(ii, 0, n * 2, pts, 0)
            }
            while (true) {
                val o = enc.dequeueOutputBuffer(info, 0)
                if (o == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                    sink.onFormat(false, enc.outputFormat)
                    continue
                }
                if (o < 0) break
                val buf = enc.getOutputBuffer(o)
                if (buf != null && info.size > 0 && info.flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG == 0) {
                    buf.position(info.offset)
                    buf.limit(info.offset + info.size)
                    sink.onSample(false, buf, info)
                }
                enc.releaseOutputBuffer(o, false)
            }
        }
    }

    /** 停下：视频发 EOS 等最后几帧出来，音频停麦克风。阻塞到两条线程都退出 */
    fun stop() {
        try {
            venc.signalEndOfInputStream()
        } catch (_: Exception) {
        }
        vThread?.join(1500)
        running = false
        vThread?.join(500)
        aThread?.join(500)
        try {
            venc.stop()
        } catch (_: Exception) {
        }
        venc.release()
        surface.release()
        try {
            mic?.stop()
        } catch (_: Exception) {
        }
        mic?.release()
        try {
            aenc?.stop()
        } catch (_: Exception) {
        }
        aenc?.release()
    }
}

/**
 * 录像落盘：`MediaMuxer` 收齐要的轨（画面，有麦克风时再加声音）才开写；开写之前到的包先攒着（拷一份），
 * 第一包画面一定是关键帧（编码器开头就是）
 */
internal class Mp4Sink(private val file: File, private val withAudio: Boolean) : AvEncoder.Sink {
    private val muxer = MediaMuxer(file.absolutePath, MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4)
    private var vTrack = -1
    private var aTrack = -1
    private var started = false
    private var stopped = false
    private val early = ArrayList<Triple<Boolean, ByteArray, MediaCodec.BufferInfo>>()
    private var startUs = -1L
    var durationUs = 0L
        private set

    @Synchronized
    override fun onFormat(video: Boolean, format: MediaFormat) {
        if (started || stopped) return
        if (video && vTrack < 0) vTrack = muxer.addTrack(format)
        if (!video && aTrack < 0 && withAudio) aTrack = muxer.addTrack(format)
        if (vTrack >= 0 && (!withAudio || aTrack >= 0)) {
            muxer.start()
            started = true
            for ((v, bytes, info) in early) write(v, ByteBuffer.wrap(bytes), info)
            early.clear()
        }
    }

    /** 没有声音轨（麦克风开不起来）：只用画面开写 */
    @Synchronized
    fun audioFailed() {
        if (started || stopped || vTrack < 0) return
        muxer.start()
        started = true
        for ((v, bytes, info) in early) if (v) write(v, ByteBuffer.wrap(bytes), info)
        early.clear()
    }

    @Synchronized
    override fun onSample(video: Boolean, buf: ByteBuffer, info: MediaCodec.BufferInfo) {
        if (stopped) return
        if (!started) {
            if (early.size < 300) {
                val b = ByteArray(info.size)
                buf.get(b)
                early += Triple(video, b, MediaCodec.BufferInfo().apply { set(0, info.size, info.presentationTimeUs, info.flags) })
            }
            return
        }
        write(video, buf, info)
    }

    private fun write(video: Boolean, buf: ByteBuffer, info: MediaCodec.BufferInfo) {
        val t = if (video) vTrack else aTrack
        if (t < 0) return
        if (startUs < 0) startUs = info.presentationTimeUs
        durationUs = maxOf(durationUs, info.presentationTimeUs - startUs)
        try {
            muxer.writeSampleData(t, buf, info)
        } catch (e: Exception) {
            Log.w("mini-render", "📷 写录像失败 $e")
        }
    }

    /** 收尾。返回文件是否可用（一帧都没写进去时不可用） */
    @Synchronized
    fun finish(): Boolean {
        stopped = true
        return try {
            if (started) muxer.stop()
            muxer.release()
            started && file.length() > 0
        } catch (e: Exception) {
            Log.w("mini-render", "📷 录像收尾失败 $e")
            false
        }
    }
}
