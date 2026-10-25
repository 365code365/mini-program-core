const B = require('../../utils/books.js')
const S = require('../../utils/store.js')

Page({
  data: {
    bookTitle: '',
    chapter: 0,
    total: 0,
    chapterTitle: '',
    paras: [],
    scrollTop: 0, // 受控滚动位置：恢复进度、换章回顶
    pct: 0,
    menu: false,
    marked: false,
    fontSize: 18,
    fontMin: S.FONT_MIN,
    fontMax: S.FONT_MAX,
    theme: S.theme('day'),
    themes: S.THEMES,
    taps: 0 // 正文被点了几次（测试看点击有没有穿透）
  },

  onLoad(query) {
    this.book = B.get(query.id) || B.BOOKS[0]
    const st = S.loadSettings()
    const p = S.progressOf(this.book.id)
    // 指定了章（目录 / 书签进来）：从那一章读；同一章时保留滚动位置
    let chapter = p ? p.chapter : 0
    let top = p ? p.scrollTop : 0
    if (query.chapter !== undefined && query.chapter !== '') {
      const c = Number(query.chapter)
      if (!p || p.chapter !== c) top = query.top ? Number(query.top) : 0
      chapter = c
    }
    this.lastTop = top
    this.setData({ bookTitle: this.book.title, total: this.book.chapters, fontSize: st.fontSize, theme: S.theme(st.theme) })
    this.open(chapter, top)
    if (!S.inShelf(this.book.id)) S.addShelf(this.book.id)
  },

  onShow() {
    this.startAt = Date.now()
    // 从目录页回来：目录改了进度就跳过去
    if (this.book && this.picked) {
      const p = S.progressOf(this.book.id)
      if (p && p.chapter !== this.data.chapter) this.open(p.chapter, 0)
      this.picked = false
    }
  },

  onHide() {
    this.flush()
  },

  onUnload() {
    this.flush()
  },

  // 记阅读时长 + 进度落盘
  flush() {
    if (this.startAt) {
      S.addReading(Math.round((Date.now() - this.startAt) / 1000))
      this.startAt = 0
    }
    this.save()
  },

  open(chapter, top) {
    const marked = S.loadBookmarks().some(m => m.bookId === this.book.id && m.chapter === chapter)
    this.lastTop = top
    // scroll-top 是受控属性，只有值变了才滚：上一章停在 0 后被手指滚走，再设 0 不会回顶，
    // 所以与上次设的值相同就错开 0.1px（与微信上的常见写法一致）
    const st = top === this.data.scrollTop ? top + 0.1 : top
    this.setData({
      chapter,
      chapterTitle: B.chapterTitle(this.book, chapter),
      paras: B.content(this.book, chapter),
      scrollTop: st,
      marked
    })
    this.save()
  },

  save() {
    const ratio = this.maxTop ? this.lastTop / this.maxTop : 0
    const p = S.saveProgress(this.book.id, this.data.chapter, this.lastTop || 0, ratio)
    if (p.pct !== this.data.pct) this.setData({ pct: p.pct })
  },

  // 滚动只记数，300ms 落一次盘（不在每个 scroll 事件里写 storage）
  onScroll(e) {
    this.lastTop = e.detail.scrollTop
    this.maxTop = Math.max(1, e.detail.scrollHeight - this.viewH())
    const now = Date.now()
    if (!this.savedAt || now - this.savedAt > 300) {
      this.savedAt = now
      this.save()
    }
    clearTimeout(this.tail)
    this.tail = setTimeout(() => this.save(), 320)
  },

  viewH() {
    if (!this.winH) this.winH = wx.getSystemInfoSync().windowHeight
    return this.winH
  },

  // 点正文：菜单开着 → 收起；关着 → 只有点中间那一块（上下各留 30%）才呼出
  onTapContent(e) {
    this.setData({ taps: this.data.taps + 1 })
    if (this.data.menu) {
      this.setData({ menu: false })
      return
    }
    const t = e.changedTouches && e.changedTouches[0]
    const y = t ? t.clientY : this.viewH() / 2
    const h = this.viewH()
    if (y > h * 0.3 && y < h * 0.7) this.setData({ menu: true })
  },

  onPrev() {
    if (this.data.chapter === 0) {
      wx.showToast({ title: '已经是第一章了', icon: 'none' })
      return
    }
    this.open(this.data.chapter - 1, 0)
  },

  onNext() {
    if (this.data.chapter >= this.data.total - 1) {
      wx.showToast({ title: '已经是最后一章了', icon: 'none' })
      return
    }
    this.open(this.data.chapter + 1, 0)
  },

  // 字号 slider：e.detail.value 是 number
  onFont(e) {
    const fontSize = e.detail.value
    this.setData({ fontSize })
    S.saveSettings(Object.assign(S.loadSettings(), { fontSize }))
  },

  onTheme(e) {
    const theme = S.theme(e.detail.value)
    this.setData({ theme })
    S.saveSettings(Object.assign(S.loadSettings(), { theme: theme.key }))
  },

  onBookmark() {
    const ok = S.addBookmark({
      bookId: this.book.id,
      bookTitle: this.book.title,
      chapter: this.data.chapter,
      chapterTitle: this.data.chapterTitle,
      top: Math.round(this.lastTop || 0),
      excerpt: this.data.paras[0].slice(0, 20) + '…'
    })
    if (ok) this.setData({ marked: true })
    wx.showToast({ title: ok ? '已添加书签' : '本章已有书签', icon: ok ? 'success' : 'none' })
  },

  onCatalog() {
    this.picked = true
    this.save()
    wx.navigateTo({ url: '/pages/catalog/catalog?from=reader&id=' + this.book.id })
  },

  onMine() {
    wx.switchTab({ url: '/pages/mine/mine' })
  },

  onBack() {
    wx.navigateBack()
  }
})
