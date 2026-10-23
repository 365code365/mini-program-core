package dev.minirender

import android.app.Activity
import android.content.Context
import android.util.Log
import org.json.JSONObject
import java.util.concurrent.atomic.AtomicBoolean

/**
 * `hostCall` 事件 → 宿主登记的 [MiniHostApi] / SDK 自带的媒体那一组（[MiniMediaApis]）→ 回包。
 * 与 Flutter 插件 `controller.dart` 的 `_onHostCall` 同一条规则：宿主登记的优先；
 * 没登记、又不是自带的回 `<api>:fail 宿主没有实现`（只会出现在名字点了、处理器又被拿掉的时候）。主线程上调。
 */
internal class HostCallRouter(context: Context, private val session: EngineSession) {
    val apis = LinkedHashMap<String, MiniHostApi>()
    var builtin = true
    private val media by lazy { MiniMediaApis(context) }

    /** 建引擎时交给 `hostApis` 选项的名字：点了名的 `wx.*` 才交给宿主，没点名的照旧立刻 fail */
    fun names(): List<String> = (apis.keys + if (builtin) MiniMediaApis.NAMES else emptyList()).distinct()

    fun dispatch(ev: JSONObject, activity: Activity?) {
        val call = MiniHostCall.fromJson(ev)
        val reply = replyFor(call)
        val api = apis[call.api]
        when {
            api != null -> try {
                api.handle(call, reply)
            } catch (e: Exception) {
                Log.w(TAG, "hostApis[${call.api}] 抛了异常：$e")
                reply.fail("${call.api}:fail ${e.message ?: e}")
            }
            builtin && call.api in MiniMediaApis.NAMES -> media.handle(activity, call, reply)
            else -> reply.fail("${call.api}:fail 宿主没有实现")
        }
    }

    private fun replyFor(call: MiniHostCall) = object : MiniHostReply {
        private val done = AtomicBoolean(false)

        override fun success(result: JSONObject) = send(true, result)

        override fun fail(errMsg: String, extra: JSONObject?) =
            send(false, JSONObject(extra?.toString() ?: "{}").put("errMsg", errMsg))

        private fun send(ok: Boolean, res: JSONObject) {
            if (!done.compareAndSet(false, true)) return
            // 回包在引擎线程上交（success / fail 回调里的 setData 由这一帧画出来）
            session.run { MiniEngine.nativeHostReply(it, call.id, ok, res.toString()) }
        }
    }

    private companion object {
        const val TAG = "mini-render"
    }
}
