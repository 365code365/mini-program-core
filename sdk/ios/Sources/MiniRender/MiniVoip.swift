import Foundation

/// `<voip-room>` 的通话接口，交给宿主自己的音视频后台（与 Android `MiniVoip.kt` 同一套约定）。
///
/// 微信的 `wx.joinVoIPChat` 要微信后台签的 `signature` / `groupId`，媒体也走微信的服务器，在微信之外进不了微信的房间。
/// SDK 只接好小程序这一侧：`joinVoIPChat` / `exitVoIPChat` / `updateVoIPChatMuteConfig` / `subscribeVoIPVideoMembers` 放进宿主的
/// `hostApis` 自己实现（宿主有 `joinVoIPChat` 时引擎不再替 `mode="video"` 派「进不了房」的 binderror）；`onVoIPChatMembersChanged`
/// 这类事件用 `dispatchEvent("voIPChatMembersChanged", …)` 推；`<voip-room mode="video" openid>` 那一格向 [video] 要这个人的画面。
public enum MiniVoip {
    /// 一路远端画面的接收端：RGBA（`stride` 字节一行，从上往下），只在这次调用里有效
    public typealias Sink = (_ rgba: UnsafePointer<UInt8>, _ width: Int, _ height: Int, _ stride: Int) -> Void

    /// 宿主提供：开始把 `openId` 的画面按 `width×height`（设备像素，就是框的大小）交给 `sink`，返回停止用的闭包
    public static var video: ((_ openId: String, _ width: Int, _ height: Int, _ sink: @escaping Sink) -> () -> Void)?
}
