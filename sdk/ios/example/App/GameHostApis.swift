import MiniRender
import UIKit

/// 示例 App 给小游戏接的「宿主能力」（创建选项 hostApis 里点名的那些）：虚拟支付、激励视频、分享、登录。
/// 与 Android 示例的 GameHostApis.kt 一一对应。
///
/// 都是**演示实现**：支付弹一个确认框（不扣钱）、广告是一个 5 秒倒计时的全屏页、分享弹提示、登录给一个假 code。
/// 正式接入时把这几个分支换成米大师 / 广告 SDK / 微信 OpenSDK 的真实调用，游戏侧代码不用改 ——
/// 游戏只认 wx.requestMidasPayment 的 success / fail 与广告的 onLoad / onClose({isEnded})。
///
/// 协议：引擎发宿主事件 {"type":"hostCall","id","api","args"}；处理完 `MiniGameView.hostReply` 回 success / fail；
/// 对象事件（广告的 load / close）用 `dispatchEvent("contextEvent", {context, contextId, event, data})` 推。
final class GameHostApis {
    static let apis = ["requestMidasPayment", "checkIsSupportMidasPayment", "shareAppMessage", "login",
                       "RewardedVideoAd.create", "RewardedVideoAd.load", "RewardedVideoAd.show", "RewardedVideoAd.destroy"]

    private weak var presenter: UIViewController?
    private let view: () -> MiniGameView?

    init(presenter: UIViewController, view: @escaping () -> MiniGameView?) {
        self.presenter = presenter
        self.view = view
    }

    /// 控制器的 onHostEvent 里收到 hostCall 就交给这里；不认识的返回 false
    func handle(_ call: [String: Any]) -> Bool {
        let id = (call["id"] as? NSNumber)?.intValue ?? 0
        let args = call["args"] as? [String: Any] ?? [:]
        let ctx = args["contextId"].map { "\($0)" } ?? ""
        switch call["api"] as? String ?? "" {
        case "checkIsSupportMidasPayment": ok(id, ["isSupport": true])
        case "requestMidasPayment": pay(id, args)
        case "shareAppMessage":
            StoreSheets.toast("（演示）已分享：\(args["title"] as? String ?? "")")
            ok(id)
        case "login": ok(id, ["code": "demo_game_" + String(Int(Date().timeIntervalSince1970 * 1000), radix: 36)])
        case "RewardedVideoAd.create", "RewardedVideoAd.load":
            ok(id)
            adEvent(ctx, "load")
        case "RewardedVideoAd.show": showAd(id, ctx)
        case "RewardedVideoAd.destroy": ok(id)
        default: return false
        }
        return true
    }

    private func ok(_ id: Int, _ res: [String: Any] = [:]) { view()?.hostReply(id, ok: true, result: res) }

    private func fail(_ id: Int, _ msg: String, _ code: Int) { view()?.hostReply(id, ok: false, result: ["errMsg": msg, "errCode": code]) }

    private func adEvent(_ ctx: String, _ event: String, _ data: [String: Any] = [:]) {
        let o: [String: Any] = ["context": "RewardedVideoAd", "contextId": ctx, "event": event, "data": data]
        let json = (try? JSONSerialization.data(withJSONObject: o)).map { String(decoding: $0, as: UTF8.self) } ?? "{}"
        view()?.dispatchEvent("contextEvent", json: json)
    }

    /// 支付：buyQuantity 是游戏币数（1 元 = 10 游戏币，与游戏的 MIDAS.rate 一致）
    private func pay(_ id: Int, _ args: [String: Any]) {
        let yuan = Double((args["buyQuantity"] as? NSNumber)?.intValue ?? 0) / 10
        let a = UIAlertController(title: "模拟支付", message: String(format: "示例 App 没有接入米大师。\n模拟支付 ¥%.2f？（不会真的扣款）", yuan), preferredStyle: .alert)
        a.addAction(UIAlertAction(title: "取消", style: .cancel) { [weak self] _ in self?.fail(id, "requestMidasPayment:fail cancel", 1) })
        a.addAction(UIAlertAction(title: "支付", style: .default) { [weak self] _ in self?.ok(id, ["errMsg": "requestMidasPayment:ok"]) })
        presenter?.present(a, animated: true)
    }

    /// 激励视频：全屏 5 秒倒计时，到点才能「关闭并领奖」；提前关 = isEnded false
    private func showAd(_ id: Int, _ ctx: String) {
        guard let root = presenter?.view else { return fail(id, "RewardedVideoAd.show:fail no view", 1) }
        let panel = UIView(frame: root.bounds)
        panel.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        panel.backgroundColor = UIColor(red: 20 / 255, green: 24 / 255, blue: 40 / 255, alpha: 0.94)
        let title = label("（演示）激励视频广告", 20, .white)
        let count = label("5", 48, UIColor(red: 1, green: 209 / 255, blue: 102 / 255, alpha: 1))
        let close = UIButton(type: .system)
        close.backgroundColor = UIColor(red: 90 / 255, green: 90 / 255, blue: 110 / 255, alpha: 1)
        close.setTitleColor(.white, for: .normal)
        close.titleLabel?.font = .systemFont(ofSize: 16)
        close.contentEdgeInsets = UIEdgeInsets(top: 12, left: 24, bottom: 12, right: 24)
        let stack = UIStackView(arrangedSubviews: [title, count, close])
        stack.axis = .vertical
        stack.alignment = .center
        stack.spacing = 24
        stack.translatesAutoresizingMaskIntoConstraints = false
        panel.addSubview(stack)
        NSLayoutConstraint.activate([stack.centerXAnchor.constraint(equalTo: panel.centerXAnchor), stack.centerYAnchor.constraint(equalTo: panel.centerYAnchor)])
        root.addSubview(panel)
        var left = 5
        var done = false
        func render() {
            count.text = left > 0 ? "\(left)" : "✓"
            close.setTitle(left > 0 ? "跳过（不发奖励）" : "关闭并领取奖励", for: .normal)
        }
        render()
        let timer = Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { t in
            left -= 1
            render()
            if left <= 0 { t.invalidate() }
        }
        close.addAction(for: .touchUpInside) { [weak self] in
            guard !done else { return }
            done = true
            timer.invalidate()
            panel.removeFromSuperview()
            self?.adEvent(ctx, "close", ["isEnded": left <= 0])
        }
        ok(id)
    }

    private func label(_ s: String, _ size: CGFloat, _ color: UIColor) -> UILabel {
        let l = UILabel()
        l.text = s
        l.font = .systemFont(ofSize: size, weight: .semibold)
        l.textColor = color
        l.textAlignment = .center
        return l
    }
}

private extension UIControl {
    /// iOS 13 没有 addAction(UIAction)：用一个挂在控件上的闭包对象
    final class Handler: NSObject {
        let f: () -> Void
        init(_ f: @escaping () -> Void) { self.f = f }
        @objc func fire() { f() }
    }

    func addAction(for e: UIControl.Event, _ f: @escaping () -> Void) {
        let h = Handler(f)
        addTarget(h, action: #selector(Handler.fire), for: e)
        objc_setAssociatedObject(self, Unmanaged.passUnretained(h).toOpaque(), h, .OBJC_ASSOCIATION_RETAIN)
    }
}
