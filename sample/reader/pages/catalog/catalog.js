const B = require('../../utils/books.js')
const S = require('../../utils/store.js')

Page({
  data: {
    title: '',
    list: [],
    current: 0, // 当前在读的章（高亮）
    intoView: '', // scroll-into-view 的目标 id
    reverse: false,
    total: 0
  },

  onLoad(query) {
    this.book = B.get(query.id) || B.BOOKS[0]
    this.fromReader = query.from === 'reader'
    wx.setNavigationBarTitle({ title: '目录 · ' + this.book.title })
    const p = S.progressOf(this.book.id)
    const current = p ? p.chapter : 0
    this.setData({ title: this.book.title, list: B.catalog(this.book), current, total: this.book.chapters })
    // 等列表渲染出来再定位：往前留两章，让当前章不贴着顶
    setTimeout(() => this.locate(), 0)
  },

  locate() {
    this.setData({ intoView: 'ch-' + Math.max(0, this.data.current - 2) })
  },

  onLocate() {
    this.setData({ intoView: '' })
    this.locate()
  },

  onReverse() {
    this.setData({ reverse: !this.data.reverse, list: this.data.list.slice().reverse() })
  },

  // 从阅读页进来：改进度后返回（阅读页 onShow 读进度）；从详情页进来：打开阅读页
  onPick(e) {
    const i = Number(e.currentTarget.dataset.i)
    if (this.fromReader) {
      S.saveProgress(this.book.id, i, 0, 0)
      wx.navigateBack()
    } else {
      wx.redirectTo({ url: '/pages/reader/reader?id=' + this.book.id + '&chapter=' + i })
    }
  }
})
