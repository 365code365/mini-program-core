import MiniRender
import UIKit

/// 示例的 hostApis：引擎做不了、SDK 也没自带，要 App 自己接的那些（登录、支付、定位、扫码……）。
/// 与 Flutter 示例的 `example/lib/host_apis.dart`、Android 示例的 `DemoHostApis.kt` 一一对应（同一组 API、
/// 同样的演示数据），demo 小程序的「宿主」tab 在三个示例里结果一样。
///
/// 真实 App 在这里接自己的账号体系、支付 SDK、地图 / 定位 SDK、扫码库。要用户点头的（支付、拨号、授权……）
/// 弹宿主自己的对话框 —— 表明调用确实走到了宿主、结果是宿主给的。用户取消一律 `<api>:fail cancel`（与微信一致）。
///
final class DemoHostApis {
    private weak var presenter: UIViewController?
    /// wx.navigateToMiniProgram 的参数（appId / path / extraData），用户点了允许之后交给宿主去打开
    private let onNavigateToMiniProgram: ([String: Any]) -> Void
    private var granted: Set<String> = ["scope.writePhotosAlbum", "scope.camera"]

    private static let scopes: [String: String] = [
        "scope.userLocation": "位置信息", "scope.userLocationBackground": "后台定位", "scope.record": "麦克风",
        "scope.camera": "摄像头", "scope.writePhotosAlbum": "保存到相册", "scope.address": "通讯地址",
        "scope.invoiceTitle": "发票抬头", "scope.werun": "微信运动步数", "scope.bluetooth": "蓝牙",
    ]
    private static let user: [String: Any] = [
        "nickName": "原生 iOS 用户", "avatarUrl": "", "gender": 0, "language": "zh_CN", "country": "", "province": "", "city": "",
    ]
    private static let demoCipher: [String: Any] = ["encryptedData": "ZGVtbw==", "iv": "ZGVtbw==", "cloudID": "demo_cloud_id"]

    init(presenter: UIViewController, onNavigateToMiniProgram: @escaping ([String: Any]) -> Void = { _ in }) {
        self.presenter = presenter
        self.onNavigateToMiniProgram = onNavigateToMiniProgram
    }

    var apis: [String: MiniHostApi] {
        [
            // ── 账号与授权 ──
            "login": { _, r in r.success(["code": "demo_" + String(Int(Date().timeIntervalSince1970 * 1000), radix: 36)]) },
            "checkSession": { _, r in r.success() },
            "getUserProfile": ask({ ("授权个人信息", "小程序申请获取你的昵称、头像\n用途：\($0.args["desc"] as? String ?? "")", "允许") },
                                  fail: "auth deny") { _ in ["userInfo": Self.user] },
            "getUserInfo": { _, r in
                let raw = (try? JSONSerialization.data(withJSONObject: Self.user)).flatMap { String(data: $0, encoding: .utf8) } ?? "{}"
                r.success(["userInfo": Self.user, "rawData": raw, "signature": "demo_signature", "encryptedData": "ZGVtbw==", "iv": "ZGVtbw=="])
            },
            "authorize": ask({ c in
                let scope = c.args["scope"] as? String ?? ""
                return ("授权", "小程序申请使用：\(Self.scopes[scope] ?? scope)（\(scope)）", "允许")
            }, fail: "auth deny") { [weak self] c in
                self?.granted.insert(c.args["scope"] as? String ?? "")
                return [:]
            },
            "openSetting": { [weak self] _, r in
                let set = self?.granted ?? []
                r.success(["authSetting": Dictionary(uniqueKeysWithValues: Self.scopes.keys.map { ($0, set.contains($0)) })])
            },
            // ── 位置与地址 ──
            "getLocation": { c, r in
                r.success(["latitude": 22.5431, "longitude": 114.0579, "speed": 0, "accuracy": 30, "altitude": 0,
                           "verticalAccuracy": 0, "horizontalAccuracy": 30, "type": c.args["type"] as? String ?? "wgs84"])
            },
            "chooseLocation": { [weak self] c, r in self?.chooseLocation(c, r) ?? r.fail("chooseLocation:fail cancel") },
            "openLocation": show { c in
                ("查看位置", "\(c.args["name"] ?? "位置")\n\(c.args["address"] ?? "")\n\(c.args["latitude"] ?? ""), \(c.args["longitude"] ?? "")")
            },
            "chooseAddress": ask({ _ in ("选择收货地址", "张三 13800000000\n广东省深圳市南山区科技园 1 号", "使用这个地址") }) { _ in
                ["userName": "张三", "postalCode": "518057", "provinceName": "广东省", "cityName": "深圳市", "countyName": "南山区",
                 "streetName": "粤海街道", "detailInfo": "科技园 1 号", "detailInfoNew": "科技园 1 号", "nationalCode": "440305",
                 "telNumber": "13800000000"]
            },
            // ── 支付与消息 ──
            "requestPayment": ask({ c in
                let total = (c.args["total"] as? NSNumber)?.doubleValue ?? 0
                return ("模拟支付", String(format: "小程序请求支付 ¥%.2f", total / 100), "支付")
            }) { _ in [:] },
            "requestSubscribeMessage": { [weak self] c, r in
                let ids = (c.args["tmplIds"] as? [Any] ?? []).map { "\($0)" }
                self?.confirm("订阅消息", "小程序想给你发送 \(ids.count) 类通知", "允许") { ok in
                    r.success(Dictionary(uniqueKeysWithValues: ids.map { ($0, ok ? "accept" : "reject") }))
                }
            },
            "chooseInvoice": ask({ _ in ("选择发票抬头", "示例科技有限公司\n税号 91440300000000000X", "使用") }) { _ in
                ["invoiceInfo": #"{"title":"示例科技有限公司","taxNumber":"91440300000000000X","type":0}"#]
            },
            "openCustomerServiceChat": show { c in
                ("客服会话", "（演示）宿主在这里打开企业微信客服：\((c.args["extInfo"] as? [String: Any])?["url"] ?? "")")
            },
            // ── 设备 ──
            "scanCode": { _, r in r.success(["result": "https://example.com/qr/demo", "scanType": "QR_CODE", "charSet": "UTF-8", "path": ""]) },
            "makePhoneCall": ask({ c in ("拨打电话", "\(c.args["phoneNumber"] ?? "")", "呼叫") }) { _ in [:] },
            "chooseContact": { _, r in
                r.success(["phoneNumber": "13800000000", "displayName": "李四", "phoneNumberList": ["13800000000", "010-12345678"]])
            },
            "addPhoneContact": ask({ c in
                ("添加到通讯录", "\(c.args["lastName"] ?? "")\(c.args["firstName"] ?? "") \(c.args["mobilePhoneNumber"] ?? "")", "添加")
            }) { _ in [:] },
            "startSoterAuthentication": { [weak self] c, r in
                self?.confirm("生物认证", "\(c.args["authContent"] ?? "请验证")（演示：点「验证」当作通过）", "验证") { ok in
                    guard ok else { return r.fail("startSoterAuthentication:fail cancel", extra: ["errCode": 90008]) }
                    let mode = (c.args["requestAuthModes"] as? [Any])?.first.map { "\($0)" } ?? "fingerPrint"
                    r.success(["authMode": mode, "resultJSON": #"{"raw":"\#(c.args["challenge"] ?? "")","fid":"demo"}"#,
                               "resultJSONSignature": "demo_signature", "errCode": 0])
                }
            },
            "getWeRunData": { _, r in r.success(Self.demoCipher) },
            // ── 跳转与分享 ──
            // 先把 success 回给小程序，再换小程序（换的时候会关掉当前这个，回调就没人接了）。
            // 目标小程序拿到的启动参数：scene 1037 + referrerInfo
            "navigateToMiniProgram": ask({ c in
                let path = c.args["path"] as? String ?? ""
                return ("打开小程序", "即将打开 \(c.args["appId"] ?? "")\n路径 \(path.isEmpty ? "首页" : path)", "允许")
            }) { [weak self] c in
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) { self?.onNavigateToMiniProgram(c.args) }
                return [:]
            },
            // 只有被别的小程序打开时才回得去（与微信一致）；例子里小程序是从原生列表打开的
            "navigateBackMiniProgram": { _, r in r.fail("navigateBackMiniProgram:fail 当前小程序不是由其它小程序打开的") },
            "getShareInfo": { _, r in r.success(Self.demoCipher) },
        ]
    }

    /// 宿主自己的选点界面（演示数据：以传进来的坐标为中心的四个地点），取消回 `chooseLocation:fail cancel`
    private func chooseLocation(_ c: MiniHostCall, _ r: MiniHostReply) {
        guard let vc = presenter, vc.viewIfLoaded?.window != nil else { return r.fail("chooseLocation:fail cancel") }
        let lat = (c.args["latitude"] as? NSNumber)?.doubleValue ?? 39.908823
        let lng = (c.args["longitude"] as? NSNumber)?.doubleValue ?? 116.39747
        let pois: [(String, String, Double, Double)] = [
            ("天安门广场", "北京市东城区东长安街", 0, 0),
            ("故宫博物院", "北京市东城区景山前街 4 号", 0.0094, 0),
            ("国家博物馆", "北京市东城区东长安街 16 号", -0.0012, 0.0043),
            ("人民大会堂", "北京市西城区西长安街", -0.0012, -0.0044),
        ]
        let a = UIAlertController(title: "选择位置（宿主界面，演示数据）", message: nil, preferredStyle: .actionSheet)
        for p in pois {
            a.addAction(UIAlertAction(title: "\(p.0)  \(p.1)", style: .default) { _ in
                r.success(["name": p.0, "address": p.1, "latitude": lat + p.2, "longitude": lng + p.3])
            })
        }
        a.addAction(UIAlertAction(title: "取消", style: .cancel) { _ in r.fail("chooseLocation:fail cancel") })
        vc.present(a, animated: true)
    }

    // MARK: - 三种形状：问一句再回 / 只展示 / 直接回

    /// 弹确认框，用户点了才 `success(result(call))`；取消回 `<api>:fail <fail>`
    private func ask(_ prompt: @escaping (MiniHostCall) -> (String, String, String), fail: String = "cancel",
                     _ result: @escaping (MiniHostCall) -> [String: Any]) -> MiniHostApi {
        { [weak self] c, r in
            let (title, msg, ok) = prompt(c)
            guard let self else { return r.fail("\(c.api):fail host gone") }
            self.confirm(title, msg, ok) { yes in
                yes ? r.success(result(c)) : r.fail("\(c.api):fail \(fail)")
            }
        }
    }

    /// 只有「好」一个按钮，点完回 success
    private func show(_ prompt: @escaping (MiniHostCall) -> (String, String)) -> MiniHostApi {
        { [weak self] c, r in
            let (title, msg) = prompt(c)
            self?.alert(title, msg, ok: "好", cancel: false) { _ in r.success() } ?? r.success()
        }
    }

    private func confirm(_ title: String, _ msg: String, _ ok: String, _ done: @escaping (Bool) -> Void) {
        alert(title, msg, ok: ok, cancel: true, done) ?? done(false)
    }

    /// 返回 nil 表示没地方弹（控制器已经没了）
    @discardableResult
    private func alert(_ title: String, _ msg: String, ok: String, cancel: Bool, _ done: @escaping (Bool) -> Void) -> Void? {
        guard let vc = presenter, vc.viewIfLoaded?.window != nil else { return nil }
        let a = UIAlertController(title: title, message: msg, preferredStyle: .alert)
        if cancel { a.addAction(UIAlertAction(title: "取消", style: .cancel) { _ in done(false) }) }
        a.addAction(UIAlertAction(title: ok, style: .default) { _ in done(true) })
        vc.present(a, animated: true)
        return ()
    }
}
