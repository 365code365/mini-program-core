const B = require('../../utils/books.js')
const S = require('../../utils/store.js')

const FOLD = 48 // 收起时显示的字数

Page({
  data: {
    book: null,
    expanded: false,
    introText: '',
    foldable: false,
    inShelf: false,
    progress: null, // { chapter, pct, chapterTitle }
    firstChapters: []
  },

  onLoad(query) {
    const book = B.get(query.id) || B.BOOKS[0]
    this.book = book
    wx.setNavigationBarTitle({ title: book.title })
    this.setData({
      book: { id: book.id, title: book.title, author: book.author, category: book.category, score: book.score, words: book.words, reads: book.reads, chapters: book.chapters, c0: book.c0, c1: book.c1 },
      foldable: book.intro.length > FOLD,
      firstChapters: B.catalog(book).slice(0, 3)
    })
    this.fold()
  },

  onShow() {
    const p = S.progressOf(this.book.id)
    this.setData({
      inShelf: S.inShelf(this.book.id),
      progress: p ? { chapter: p.chapter, pct: p.pct, chapterTitle: B.chapterTitle(this.book, p.chapter) } : null
    })
  },

  fold() {
    const t = this.book.intro
    this.setData({ introText: this.data.expanded || t.length <= FOLD ? t : t.slice(0, FOLD) + '…' })
  },

  onToggle() {
    this.setData({ expanded: !this.data.expanded })
    this.fold()
  },

  onShelf() {
    if (this.data.inShelf) {
      wx.showToast({ title: '已在书架中', icon: 'none' })
      return
    }
    S.addShelf(this.book.id)
    this.setData({ inShelf: true })
    wx.showToast({ title: '已加入书架', icon: 'success' })
  },

  // 开始阅读 / 继续阅读：有进度就从进度处接着读
  onRead() {
    wx.navigateTo({ url: '/pages/reader/reader?id=' + this.book.id })
  },

  onCatalog() {
    wx.navigateTo({ url: '/pages/catalog/catalog?id=' + this.book.id })
  },

  onChapter(e) {
    wx.navigateTo({ url: '/pages/reader/reader?id=' + this.book.id + '&chapter=' + e.currentTarget.dataset.i })
  }
})
