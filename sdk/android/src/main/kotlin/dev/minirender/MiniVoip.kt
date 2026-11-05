package dev.minirender

import java.nio.ByteBuffer

/**
 * `<voip-room>` 的通话接口，交给宿主自己的音视频后台。
 *
 * 微信的 `wx.joinVoIPChat` 要微信后台签的 `signature` / `groupId`，媒体也走微信的服务器，在微信之外进不了微信的房间。
 * SDK 只把小程序这一侧接好，房间、信令、音视频由宿主自己的后台（自建 RTC / 第三方云）提供：
 * - **接口**：`joinVoIPChat` / `exitVoIPChat` / `updateVoIPChatMuteConfig` / `subscribeVoIPVideoMembers` / `join1v1Chat`
 *   放进宿主的 `hostApis`，在 [MiniHostApi] 里实现（回包形状照官方：`{openIdList, errCode, errMsg}`）；
 *   宿主 `hostApis` 里有 `joinVoIPChat` 时，引擎不再替 `<voip-room mode="video">` 派「进不了房」的 binderror；
 * - **事件**：`onVoIPChatMembersChanged` / `onVoIPChatSpeakersChanged` / `onVoIPChatInterrupted` / `onVoIPVideoMembersChanged`
 *   用 [MiniProgramView.dispatchEvent]（名字是 `voIPChatMembersChanged` 这类，去掉 on、首字母小写）推给逻辑层；
 * - **画面**：`<voip-room mode="video" openid="…">` 那一格向 [video] 要这个人的画面：宿主按给的宽高（设备像素，就是框的大小）
 *   交 RGBA（紧密排列、从上往下），SDK 交给引擎画进页面（同层渲染，滚动时跟着页面走）。没设 [video] 时那一格是黑底；
 *   `mode="camera"`（自己）照旧是本机相机。
 */
object MiniVoip {
    /** 一路远端画面的接收端：`rgba` 只在这次调用里有效 */
    fun interface Sink {
        fun frame(rgba: ByteBuffer, width: Int, height: Int)
    }

    /** 宿主提供：开始把 [openId] 的画面按 `width×height` 交给 [sink]，返回停止用的句柄（框变了、组件拿掉时调） */
    fun interface VideoSource {
        fun attach(openId: String, width: Int, height: Int, sink: Sink): AutoCloseable
    }

    @Volatile var video: VideoSource? = null
}
