import UIKit

/// 搜索小程序（照微信「搜一搜 · 小程序」），与 Android 示例 SearchActivity.kt 一一对应：
/// - 没输入：搜索历史（可清空）、热门搜索（服务器按搜索次数排，前三名标色）、按分类浏览；
/// - 输入中：联想词（停 200ms 再问服务器，过期的回包丢掉）；
/// - 回车 / 点联想词 / 点历史：结果列表（名称里命中的字标绿），上面一排分类可以再筛；
/// - 从入口页点某个分类进来：直接是这个分类的结果。
/// 搜索走服务器的 `/api/apps?q=&category=`（tools/mini-store/store_search.py），与管理页同一套排序。
final class SearchViewController: UIViewController, UITextFieldDelegate {
    private let store: MiniStore
    private let local = StoreLocal()
    private var category: String
    private var categories: [(String, Int)] = []
    private let input = UITextField()
    private let body = UIStackView()
    /// 每发一次请求 +1：回包对不上就是过期的（用户又打了字）
    private var seq = 0
    private var suggestWork: DispatchWorkItem?

    init(store: MiniStore, category: String = "") {
        self.store = store
        self.category = category
        super.init(nibName: nil, bundle: nil)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .white
        input.placeholder = "搜索小程序"
        input.font = .systemFont(ofSize: 15)
        input.returnKeyType = .search
        input.clearButtonMode = .whileEditing
        input.delegate = self
        input.addTarget(self, action: #selector(textChanged), for: .editingChanged)
        let box = UIStackView()
        box.alignment = .center
        box.spacing = 8
        box.backgroundColor = UIColor(argb: 0xFFEDEDED)
        box.isLayoutMarginsRelativeArrangement = true
        box.layoutMargins = UIEdgeInsets(top: 0, left: 10, bottom: 0, right: 4)
        box.addArrangedSubview(StoreWidgets.label("🔍", 13, .black))
        box.addArrangedSubview(input)
        input.heightAnchor.constraint(equalToConstant: 36).isActive = true
        let boxWrap = UIView()
        boxWrap.backgroundColor = UIColor(argb: 0xFFEDEDED)
        boxWrap.layer.cornerRadius = 8
        boxWrap.addSubview(box)
        box.pinEdges(to: boxWrap)
        let cancel = UIButton(type: .custom)
        cancel.setTitle("取消", for: .normal)
        cancel.titleLabel?.font = .systemFont(ofSize: 15)
        cancel.setTitleColor(UIColor(argb: 0xFF576B95), for: .normal)
        cancel.contentEdgeInsets = UIEdgeInsets(top: 8, left: 12, bottom: 8, right: 12)
        cancel.setContentHuggingPriority(.required, for: .horizontal)
        cancel.addAction { [weak self] in self?.navigationController?.popViewController(animated: true) }
        let bar = UIStackView(arrangedSubviews: [boxWrap, cancel])
        bar.alignment = .center
        bar.isLayoutMarginsRelativeArrangement = true
        bar.layoutMargins = UIEdgeInsets(top: 8, left: 12, bottom: 8, right: 4)
        let scroll = UIScrollView()
        scroll.keyboardDismissMode = .onDrag
        body.axis = .vertical
        scroll.addSubview(body)
        body.translatesAutoresizingMaskIntoConstraints = false
        for v in [bar, scroll] as [UIView] {
            v.translatesAutoresizingMaskIntoConstraints = false
            view.addSubview(v)
        }
        NSLayoutConstraint.activate([
            bar.topAnchor.constraint(equalTo: view.safeAreaLayoutGuide.topAnchor),
            bar.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            bar.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            scroll.topAnchor.constraint(equalTo: bar.bottomAnchor),
            scroll.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            scroll.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            body.topAnchor.constraint(equalTo: scroll.contentLayoutGuide.topAnchor),
            body.bottomAnchor.constraint(equalTo: scroll.contentLayoutGuide.bottomAnchor),
            body.leadingAnchor.constraint(equalTo: scroll.frameLayoutGuide.leadingAnchor),
            body.trailingAnchor.constraint(equalTo: scroll.frameLayoutGuide.trailingAnchor),
        ])
        DispatchQueue.global().async { [store] in
            let c = (try? store.categories()) ?? []
            DispatchQueue.main.async { [weak self] in
                guard let self else { return }
                self.categories = c
                if !self.category.isEmpty { self.results("") } else if self.text.isEmpty { self.idle() }
            }
        }
        if !category.isEmpty { results("") } else { idle() }
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        navigationController?.setNavigationBarHidden(true, animated: animated)
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        if category.isEmpty { input.becomeFirstResponder() }
    }

    private var text: String { (input.text ?? "").trimmingCharacters(in: .whitespaces) }

    func textFieldShouldReturn(_ tf: UITextField) -> Bool {
        submit(tf.text ?? "")
        return true
    }

    @objc private func textChanged() {
        suggestWork?.cancel()
        if text.isEmpty { return idle() }
        let w = DispatchWorkItem { [weak self] in self.map { $0.suggest($0.text) } }
        suggestWork = w
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2, execute: w)
    }

    private func submit(_ q: String) {
        let w = q.trimmingCharacters(in: .whitespaces)
        if w.isEmpty, category.isEmpty { return }
        suggestWork?.cancel()
        if !w.isEmpty { local.addHistory(w) }
        if input.text != w { input.text = w }
        input.resignFirstResponder()
        results(w)
    }

    private func clear() {
        body.arrangedSubviews.forEach { $0.removeFromSuperview() }
    }

    /// 没输入：历史、热搜、分类
    private func idle() {
        seq += 1
        let my = seq
        clear()
        let hist = local.history()
        if !hist.isEmpty {
            body.addArrangedSubview(StoreWidgets.header("搜索历史", action: "清空") { [weak self] in
                self?.local.clearHistory()
                self?.idle()
            })
            body.addArrangedSubview(flow(hist.map { w in StoreWidgets.chip(w) { [weak self] in self?.submit(w) } }))
        }
        let hotSlot = UIStackView()
        hotSlot.axis = .vertical
        body.addArrangedSubview(hotSlot)
        if !categories.isEmpty {
            body.addArrangedSubview(StoreWidgets.header("按分类浏览"))
            body.addArrangedSubview(flow(categories.map { c, n in
                StoreWidgets.chip("\(c) \(n)") { [weak self] in
                    guard let self else { return }
                    self.category = c
                    self.results(self.text)
                }
            }))
        }
        DispatchQueue.global().async { [store] in
            let hot = (try? store.hotKeywords()) ?? []
            DispatchQueue.main.async { [weak self] in
                guard let self, my == self.seq, !hot.isEmpty else { return }
                hotSlot.addArrangedSubview(StoreWidgets.header("热门搜索"))
                hotSlot.addArrangedSubview(self.hotGrid(hot))
            }
        }
    }

    /// 热搜两列，前三名的序号标红橙黄（微信的样子）
    private func hotGrid(_ words: [String]) -> UIView {
        let col = UIStackView()
        col.axis = .vertical
        col.isLayoutMarginsRelativeArrangement = true
        col.layoutMargins = UIEdgeInsets(top: 0, left: 16, bottom: 0, right: 16)
        for r in stride(from: 0, to: words.count, by: 2) {
            let row = UIStackView()
            row.distribution = .fillEqually
            for c in 0..<2 {
                guard r + c < words.count else { row.addArrangedSubview(UIView()); continue }
                let w = words[r + c]
                let rank = r + c + 1
                let item = PressableRow()
                item.accessibilityLabel = w
                let h = UIStackView()
                h.alignment = .center
                h.isUserInteractionEnabled = false
                h.isLayoutMarginsRelativeArrangement = true
                h.layoutMargins = UIEdgeInsets(top: 9, left: 0, bottom: 9, right: 8)
                let colors: [Int: UInt32] = [1: 0xFFFA5151, 2: 0xFFFA9D3B, 3: 0xFFF7C543]
                let n = StoreWidgets.label("\(rank)", 14, UIColor(argb: colors[rank] ?? 0x61000000), bold: true)
                n.widthAnchor.constraint(equalToConstant: 22).isActive = true
                h.addArrangedSubview(n)
                h.addArrangedSubview(StoreWidgets.label(w, 14, UIColor(argb: 0xDE000000)))
                item.addSubview(h)
                h.pinEdges(to: item)
                item.onTap = { [weak self] in self?.submit(w) }
                row.addArrangedSubview(item)
            }
            col.addArrangedSubview(row)
        }
        return col
    }

    /// 输入中：联想词
    private func suggest(_ q: String) {
        if q.isEmpty { return }
        seq += 1
        let my = seq
        DispatchQueue.global().async { [store] in
            let words = (try? store.suggest(q)) ?? []
            DispatchQueue.main.async { [weak self] in
                guard let self, my == self.seq else { return }
                self.clear()
                // 第一行永远是「搜索 xxx」：联想不到也能直接搜
                for w in [q] + words.filter({ $0 != q }) {
                    let r = PressableRow()
                    r.accessibilityLabel = w
                    let l = UILabel()
                    l.font = .systemFont(ofSize: 15)
                    l.attributedText = StoreWidgets.highlight(w, q, base: .black)
                    r.addSubview(l)
                    l.pinEdges(to: r, insets: UIEdgeInsets(top: 13, left: 16, bottom: 13, right: 16))
                    r.onTap = { [weak self] in self?.submit(w) }
                    self.body.addArrangedSubview(r)
                }
            }
        }
    }

    private func results(_ q: String) {
        seq += 1
        let my = seq
        clear()
        body.addArrangedSubview(categoryBar(q))
        let status = StoreWidgets.label("搜索中…", 13, StoreWidgets.sub, lines: 0)
        let wrap = UIView()
        wrap.addSubview(status)
        status.pinEdges(to: wrap, insets: UIEdgeInsets(top: 10, left: 16, bottom: 4, right: 16))
        body.addArrangedSubview(wrap)
        let cat = category
        DispatchQueue.global().async { [store] in
            let r = Result { try store.search(q, category: cat) }
            DispatchQueue.main.async { [weak self] in
                guard let self, my == self.seq else { return }
                switch r {
                case let .success((apps, total)):
                    status.text = total == 0 && !q.isEmpty ? "没有找到「\(q)」相关的小程序，换个词试试"
                        : total == 0 ? "这个分类下还没有小程序" : "共 \(total) 个小程序"
                    for a in apps { self.body.addArrangedSubview(StoreWidgets.resultRow(self, store: store, app: a, q: q)) }
                case let .failure(e):
                    status.text = "搜索失败：\(e.localizedDescription)"
                }
            }
        }
    }

    /// 结果上方一排分类：「全部」+ 各分类，点了就在当前关键词下重新筛
    private func categoryBar(_ q: String) -> UIView {
        let row = UIStackView()
        row.spacing = 8
        row.isLayoutMarginsRelativeArrangement = true
        row.layoutMargins = UIEdgeInsets(top: 6, left: 12, bottom: 4, right: 12)
        for c in [""] + categories.map(\.0) {
            row.addArrangedSubview(StoreWidgets.chip(c.isEmpty ? "全部" : c, selected: c == category) { [weak self] in
                self?.category = c
                self?.results(q)
            })
        }
        let s = UIScrollView()
        s.showsHorizontalScrollIndicator = false
        s.addSubview(row)
        row.pinEdges(to: s)
        row.heightAnchor.constraint(equalTo: s.heightAnchor).isActive = true
        s.heightAnchor.constraint(equalToConstant: 6 + 30 + 4).isActive = true
        return s
    }

    private func flow(_ views: [UIView]) -> UIView {
        let f = FlowLayout()
        f.setItems(views)
        return f
    }
}
