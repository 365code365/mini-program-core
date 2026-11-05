package dev.minirender

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.content.pm.PackageManager
import android.os.Handler
import android.os.Looper

/**
 * `<camera>` / `<live-pusher>` 要的运行时权限（CAMERA、RECORD_AUDIO）：借 [MiniMediaActivity] 弹系统的权限框，宿主什么都不用接。
 * 宿主清单里没声明的权限不申请（与 SDK 的选图同一条规矩：权限清单由宿主决定），直接回「没声明」。
 */
internal object CameraPermissions {
    private val main = Handler(Looper.getMainLooper())

    /** 正在弹权限框：宿主那一下「进后台」是这个框盖上来，相机不该当成被中断（不关、不发 bindstop） */
    @Volatile var requesting = false
        private set

    fun granted(ctx: Context, perm: String) = ctx.checkSelfPermission(perm) == PackageManager.PERMISSION_GRANTED

    /** [done] 在主线程上回：(拿到了, 没拿到的原因) */
    fun ensure(ctx: Context, perm: String, done: (Boolean, String?) -> Unit) {
        if (granted(ctx, perm)) {
            main.post { done(true, null) }
            return
        }
        if (!MiniMediaActivity.declares(ctx, perm)) {
            main.post { done(false, "宿主清单里没有声明 $perm") }
            return
        }
        main.post {
            requesting = true
            MiniMediaActivity.launch(ctx, activityOf(ctx), MiniMediaActivity.Request(perm, null) { outcome, close ->
                close()
                // 代办 Activity 关掉、宿主回前台之后才算完（onResume 在 close 之后）
                main.postDelayed({ requesting = false }, 600)
                val ok = outcome is MiniMediaActivity.Outcome.Result && granted(ctx, perm)
                done(ok, if (ok) null else "用户没有授权 $perm")
            })
        }
    }

    fun activityOf(ctx: Context): Activity? {
        var c: Context? = ctx
        while (c is ContextWrapper) {
            if (c is Activity) return c
            c = c.baseContext
        }
        return null
    }
}
