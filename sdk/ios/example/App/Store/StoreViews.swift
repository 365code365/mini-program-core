import UIKit

/// 商店界面的小零件：颜色、圆形图标、微信式 loading 层。与 Android 示例 StoreViews.kt / StoreIcons.kt、
/// Flutter 示例 lib/store/store_pages.dart 同一套样子（尺寸 dp → pt 1:1，sp → pt 1:1）。
enum StoreViews {
    static let green = UIColor(argb: 0xFF07C160)
    private static let colors: [UInt32] = [0xFF07C160, 0xFF10AEFF, 0xFFFA9D3B, 0xFFE64340, 0xFF6467F0, 0xFF1485EE, 0xFF91D300, 0xFF576B95]

    /// 底色按 id 算（与管理页 admin.js、Android / Flutter 示例同一个哈希，几处颜色一样）
    static func color(_ id: String) -> UIColor {
        var h: UInt64 = 0
        for u in id.unicodeScalars { h = (h &* 31 &+ UInt64(u.value)) & 0xFFFF_FFFF }
        return UIColor(argb: colors[Int(h % UInt64(colors.count))])
    }

    /// 圆形图标：名字的第一个字（服务器有 icon.png 时 [IconView] 再盖上去）
    static func letterIcon(id: String, name: String, size: CGFloat) -> UILabel {
        let l = UILabel(frame: CGRect(x: 0, y: 0, width: size, height: size))
        l.text = name.first.map(String.init) ?? "?"
        l.textAlignment = .center
        l.textColor = .white
        l.font = .boldSystemFont(ofSize: size * 0.42)
        l.backgroundColor = color(id)
        l.layer.cornerRadius = size / 2
        l.layer.masksToBounds = true
        l.isAccessibilityElement = false
        return l
    }
}

extension UIColor {
    /// Android 的 0xAARRGGBB 写法：两边的颜色常量逐个对得上
    convenience init(argb: UInt32) {
        self.init(red: CGFloat((argb >> 16) & 0xFF) / 255, green: CGFloat((argb >> 8) & 0xFF) / 255,
                  blue: CGFloat(argb & 0xFF) / 255, alpha: CGFloat(argb >> 24) / 255)
    }
}

/// 商店图标：先画名字首字的圆，服务器有 icon.png 就后台拉下来盖上去（圆形裁剪，与微信一样）。
/// 内存里留一份解码好的图（NSCache），磁盘缓存在 [MiniStore.iconData]。
final class IconView: UIView {
    private static let mem: NSCache<NSString, UIImage> = {
        let c = NSCache<NSString, UIImage>()
        c.totalCostLimit = 8 << 20
        return c
    }()
    private static let pool: OperationQueue = {
        let q = OperationQueue()
        q.maxConcurrentOperationCount = 3
        q.name = "store-icon"
        return q
    }()
    private let size: CGFloat

    init(store: MiniStore, app: MiniStore.App, size: CGFloat) {
        self.size = size
        super.init(frame: CGRect(x: 0, y: 0, width: size, height: size))
        isAccessibilityElement = false
        addSubview(StoreViews.letterIcon(id: app.id, name: app.name, size: size))
        guard !app.iconUrl.isEmpty else { return }
        let img = UIImageView(frame: bounds)
        img.contentMode = .scaleAspectFill
        img.layer.cornerRadius = size / 2
        img.layer.masksToBounds = true
        addSubview(img)
        let key = "\(app.id)@\(app.version)" as NSString
        if let hit = Self.mem.object(forKey: key) { img.image = hit; return }
        Self.pool.addOperation { [weak img] in
            guard let d = store.iconData(app), let im = UIImage(data: d) else { return }
            Self.mem.setObject(im, forKey: key, cost: d.count)
            DispatchQueue.main.async { img?.image = im }
        }
    }

    /// 内置示例这类没有商店信息的：只画首字圆
    init(id: String, name: String, size: CGFloat) {
        self.size = size
        super.init(frame: CGRect(x: 0, y: 0, width: size, height: size))
        isAccessibilityElement = false
        addSubview(StoreViews.letterIcon(id: id, name: name, size: size))
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    override var intrinsicContentSize: CGSize { CGSize(width: size, height: size) }
}

/// 点开小程序时盖在上面的一层：图标 + 名字 + 三个依次变绿的点 + 下载进度。
/// [hide] 淡出后把自己拿掉、停掉动画。与 Android 示例 StoreViews.LoadingCover 一样
final class LoadingCover: UIView {
    private let iconSlot = UIView()
    private let title = UILabel()
    private let dots = (0..<3).map { _ in UIView() }
    private let dotRow = UIStackView()
    private let status = UILabel()
    private let retry = UIButton(type: .system)
    private var timer: Timer?
    private var tick = 0
    private var onRetry: (() -> Void)?

    init(id: String, name: String, onClose: @escaping () -> Void) {
        super.init(frame: .zero)
        backgroundColor = .white
        let col = UIStackView()
        col.axis = .vertical
        col.alignment = .center
        col.translatesAutoresizingMaskIntoConstraints = false
        addSubview(col)
        setIdentity(id: id, name: name)
        iconSlot.translatesAutoresizingMaskIntoConstraints = false
        iconSlot.widthAnchor.constraint(equalToConstant: 64).isActive = true
        iconSlot.heightAnchor.constraint(equalToConstant: 64).isActive = true
        title.font = .systemFont(ofSize: 17)
        title.textColor = .black
        title.textAlignment = .center
        for d in dots {
            d.backgroundColor = UIColor(argb: 0xFFD8D8D8)
            d.layer.cornerRadius = 4
            d.widthAnchor.constraint(equalToConstant: 8).isActive = true
            d.heightAnchor.constraint(equalToConstant: 8).isActive = true
            dotRow.addArrangedSubview(d)
        }
        dotRow.spacing = 8
        dotRow.accessibilityLabel = "加载中"
        status.font = .systemFont(ofSize: 12)
        status.textColor = UIColor(argb: 0x61000000)
        status.textAlignment = .center
        status.numberOfLines = 0
        retry.setTitle("重试", for: .normal)
        retry.isHidden = true
        retry.addTarget(self, action: #selector(retryTap), for: .touchUpInside)
        col.addArrangedSubview(iconSlot)
        col.setCustomSpacing(14, after: iconSlot)
        col.addArrangedSubview(title)
        col.setCustomSpacing(28, after: title)
        col.addArrangedSubview(dotRow)
        col.setCustomSpacing(12, after: dotRow)
        col.addArrangedSubview(status)
        col.addArrangedSubview(retry)
        NSLayoutConstraint.activate([
            col.centerXAnchor.constraint(equalTo: centerXAnchor),
            col.centerYAnchor.constraint(equalTo: centerYAnchor, constant: -60),
            col.leadingAnchor.constraint(greaterThanOrEqualTo: leadingAnchor, constant: 32),
            status.widthAnchor.constraint(lessThanOrEqualTo: widthAnchor, constant: -64),
        ])
        let close = UIButton(type: .system)
        close.setTitle("✕", for: .normal)
        close.titleLabel?.font = .systemFont(ofSize: 20)
        close.tintColor = UIColor(argb: 0x8A000000)
        close.accessibilityLabel = "关闭"
        close.contentEdgeInsets = UIEdgeInsets(top: 12, left: 16, bottom: 12, right: 16)
        close.translatesAutoresizingMaskIntoConstraints = false
        close.addAction(onClose)
        addSubview(close)
        NSLayoutConstraint.activate([
            close.topAnchor.constraint(equalTo: safeAreaLayoutGuide.topAnchor, constant: 12),
            close.trailingAnchor.constraint(equalTo: trailingAnchor),
        ])
        // 900ms 走完三个点（与 Android 的 ValueAnimator 0..3 / 900ms 一致）
        timer = Timer.scheduledTimer(withTimeInterval: 0.3, repeats: true) { [weak self] _ in self?.step() }
        step()
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    private func step() {
        let on = tick % 3
        tick += 1
        for (i, d) in dots.enumerated() { d.backgroundColor = i == on ? StoreViews.green : UIColor(argb: 0xFFD8D8D8) }
    }

    func progress(_ text: String) { status.text = text }

    /// 深链进来时先只知道 AppID：服务器回了最新版信息后换上真名字与图标
    func setIdentity(id: String, name: String) {
        title.text = name
        iconSlot.subviews.forEach { $0.removeFromSuperview() }
        iconSlot.addSubview(StoreViews.letterIcon(id: id, name: name, size: 64))
    }

    func error(_ text: String, onRetry: @escaping () -> Void) {
        timer?.invalidate()
        dotRow.isHidden = true
        status.textColor = UIColor(argb: 0xFFE64340)
        status.text = text
        retry.isHidden = false
        self.onRetry = onRetry
    }

    @objc private func retryTap() {
        retry.isHidden = true
        dotRow.isHidden = false
        status.textColor = UIColor(argb: 0x61000000)
        status.text = ""
        timer = Timer.scheduledTimer(withTimeInterval: 0.3, repeats: true) { [weak self] _ in self?.step() }
        onRetry?()
    }

    func hide() {
        isUserInteractionEnabled = false
        UIView.animate(withDuration: 0.2, animations: { self.alpha = 0 }, completion: { _ in
            self.timer?.invalidate()
            self.removeFromSuperview()
        })
    }

    override func removeFromSuperview() {
        timer?.invalidate()
        super.removeFromSuperview()
    }
}

extension UIControl {
    /// iOS 13 没有 `addAction(UIAction)`（14 起才有）：闭包版的 target-action
    func addAction(_ block: @escaping () -> Void) {
        let t = ClosureTarget(block)
        addTarget(t, action: #selector(ClosureTarget.fire), for: .touchUpInside)
        objc_setAssociatedObject(self, &ClosureTarget.key, t, .OBJC_ASSOCIATION_RETAIN)
    }
}

private final class ClosureTarget: NSObject {
    static var key = 0
    let block: () -> Void
    init(_ b: @escaping () -> Void) { block = b }
    @objc func fire() { block() }
}
