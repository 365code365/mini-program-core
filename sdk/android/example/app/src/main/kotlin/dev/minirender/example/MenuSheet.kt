package dev.minirender.example

import dev.minirender.MiniProgramAbout

/**
 * 胶囊「•••」的底部菜单（[dev.minirender.MiniProgramView.onAbout] 接到这里）。SDK 只留口子，展示什么由宿主定；
 * 示例照微信：顶上小程序名（点了看关于）、转发 / 收藏、重新进入 / 上传诊断日志 / 关于。
 * 内置示例没有商店链接：转发、收藏灰掉。与 Flutter 示例 main.dart 的 `_showMenu` 同一份内容。
 */
internal fun MainActivity.showCapsuleMenu(about: MiniProgramAbout) {
    val app = currentStoreApp
    val store = MiniStore(this, intent.getStringExtra(MainActivity.EXTRA_STORE_URL) ?: MiniStore.DEFAULT_URL)
    val local = StoreLocal(this)
    val fav = app != null && local.isFavorite(app.id)
    val name = app?.name ?: about.name.ifEmpty { "内置示例" }
    val icon = if (app != null) StoreIcons.view(store, app, 30f) else StoreViews.icon(this, "demo", name, 30f)
    StoreSheets.capsuleMenu(
        this, icon, name,
        subtitle = app?.let { "v${it.version}" } ?: "本地小程序",
        onHeader = { showAboutDialog(about) },
        top = listOf(
            StoreSheets.Item("↗", "转发给朋友", enabled = app != null) { StoreSheets.copyLink(this, app!!, currentRoute) },
            StoreSheets.Item(if (fav) "★" else "☆", if (fav) "取消收藏" else "添加到\n我的小程序", enabled = app != null, active = fav) {
                val now = local.toggleFavorite(app!!)
                StoreSheets.toast(this, if (now) "已添加到「我的小程序」，入口页下拉可见" else "已从「我的小程序」移除")
            },
        ),
        bottom = listOf(
            StoreSheets.Item("↻", "重新进入\n小程序") { relaunch() },
            StoreSheets.Item("⚑", "上传\n诊断日志") { uploadDiag() },
            StoreSheets.Item("ⓘ", "关于") { showAboutDialog(about) },
        ),
    )
}

private fun MainActivity.showAboutDialog(a: MiniProgramAbout) {
    android.app.AlertDialog.Builder(this)
        .setTitle(a.dialogTitle)
        .setMessage(a.dialogMessage)
        .setNeutralButton("上传诊断日志") { _, _ -> uploadDiag() }
        .setPositiveButton("确定", null)
        .show()
}
