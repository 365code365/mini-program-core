package dev.minirender

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger

/**
 * 透明的「代办」Activity：替 SDK 申请权限、打开系统的选择器 / 相机、收结果。
 *
 * 为什么要它：`startActivityForResult` 的结果只回到发起它的 Activity。宿主的 Activity 是宿主自己的
 * （示例里还是最老的 `android.app.Activity`，没有 ActivityResultRegistry），要它把 `onActivityResult` /
 * `onRequestPermissionsResult` 转给视图，集成就不再是「只用一个 View」。代办 Activity 自己发起、自己收，
 * 宿主什么都不用接 —— 只要清单里有它（AAR 自带的清单会合并进去；直接编源码的工程照示例抄两行）。
 *
 * 一次一个请求：[Request.done] 在主线程上回一次（取消、拒绝权限、没有能处理的应用也回），带一个 `close`：
 * **拷完文件再调它**。照片选择器 / 文件选择器给的 `content://` 授权只属于收结果的这个 Activity，它一关授权就收回
 * （踩过：先关再在后台拷，`Calling uid … does not have permission to access picker uri`）。拷的那一会儿透明的它还盖着，
 * 触摸落在它上面，正好不让用户在选图回包之前又点别的。
 * 进程在相机里被系统杀掉的话，回来时请求表是空的，它直接关掉（那时引擎也是重开的，没人在等）。
 */
class MiniMediaActivity : Activity() {
    internal class Request(
        /** 先要拿到的权限（null = 不用申请） */
        val permission: String?,
        /** 拿到权限之后启动的 Intent（null = 只要权限） */
        val intent: Intent?,
        /** 第二个参数 `close`：用完结果里的 URI 之后调（关掉代办 Activity；不调的话授权一直在、它一直盖着） */
        val done: (Outcome, () -> Unit) -> Unit,
    )

    internal sealed class Outcome {
        class Result(val code: Int, val data: Intent?) : Outcome()
        object Denied : Outcome()
        class NoApp(val action: String?) : Outcome()
    }

    internal companion object {
        private const val EXTRA_ID = "dev.minirender.media.request"
        private const val REQ_INTENT = 0x6d76
        private const val REQ_PERMISSION = 0x6d77
        private val seq = AtomicInteger()
        private val pending = ConcurrentHashMap<Int, Request>()

        /** 主线程调。[from] 给宿主的 Activity（没有就用 NEW_TASK 起） */
        fun launch(ctx: Context, from: Activity?, req: Request) {
            val id = seq.incrementAndGet()
            pending[id] = req
            val i = Intent(ctx, MiniMediaActivity::class.java).putExtra(EXTRA_ID, id)
            try {
                if (from != null) {
                    from.startActivity(i)
                    @Suppress("DEPRECATION")
                    from.overridePendingTransition(0, 0)
                } else {
                    ctx.startActivity(i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
                }
            } catch (e: ActivityNotFoundException) {
                // 清单里没声明代办 Activity
                pending.remove(id)?.done?.invoke(Outcome.NoApp("MiniMediaActivity（清单里没有声明）")) {}
            }
        }

        /** 宿主清单里声明了这个权限（没声明的不用申请：系统相机自己有相机权限） */
        @Suppress("DEPRECATION")
        fun declares(ctx: Context, perm: String): Boolean = try {
            ctx.packageManager.getPackageInfo(ctx.packageName, PackageManager.GET_PERMISSIONS)
                .requestedPermissions?.contains(perm) == true
        } catch (e: PackageManager.NameNotFoundException) {
            false
        }
    }

    private var id = 0

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        id = intent.getIntExtra(EXTRA_ID, 0)
        val req = pending[id] ?: return finish()
        // 重建（选择器开着时转屏 / 改了深色模式）：选择器还在，等它的结果
        if (savedInstanceState != null) return
        val perm = req.permission
        if (perm != null && checkSelfPermission(perm) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(arrayOf(perm), REQ_PERMISSION)
        } else {
            proceed(req)
        }
    }

    private fun proceed(req: Request) {
        val intent = req.intent ?: return deliver(Outcome.Result(RESULT_OK, null))
        try {
            startActivityForResult(intent, REQ_INTENT)
        } catch (e: ActivityNotFoundException) {
            deliver(Outcome.NoApp(intent.action))
        }
    }

    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<out String>, grantResults: IntArray) {
        if (requestCode != REQ_PERMISSION) return
        val req = pending[id] ?: return finish()
        if (grantResults.isNotEmpty() && grantResults.all { it == PackageManager.PERMISSION_GRANTED }) proceed(req)
        else deliver(Outcome.Denied)
    }

    @Deprecated("startActivityForResult 的老回调（android.app.Activity 没有 ActivityResultRegistry）")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        if (requestCode == REQ_INTENT) deliver(Outcome.Result(resultCode, data))
    }

    private fun deliver(o: Outcome) {
        val req = pending.remove(id) ?: return close()
        req.done(o) { runOnUiThread(::close) }
    }

    private fun close() {
        if (isFinishing || isDestroyed) return
        finish()
        @Suppress("DEPRECATION")
        overridePendingTransition(0, 0)
    }

    override fun onDestroy() {
        // 没等到结果就被关了（用户在权限框之前按了返回）：当作取消，别让小程序那边一直等
        if (isFinishing) pending.remove(id)?.done?.invoke(Outcome.Result(RESULT_CANCELED, null)) {}
        super.onDestroy()
    }
}
