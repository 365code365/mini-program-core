const B = require('../../utils/books.js')
const S = require('../../utils/store.js')

Page({
  data: {
    books: [], // [{ id, title, author, c0, c1, pct, chapterTitle }]
    recent: null, // 最近在读（进度最大的那本之外，按书架顺序第一本有进度的）
    editing: false
  },

  onShow() {
    this.load()
  },

  load() {
    const prog = S.loadProgress()
    const books = S.loadShelf().map(id => {
      const b = B.get(id)
      const p = prog[id]
      return {
        id, title: b.title, author: b.author, c0: b.c0, c1: b.c1,
        pct: p ? p.pct : 0,
        status: p ? '读到 ' + p.pct + '%' : '未读',
        chapterTitle: p ? B.chapterTitle(b, p.chapter) : ''
      }
    })
    this.setData({ books, recent: books.find(b => b.pct > 0) || null })
  },

  onOpen(e) {
    wx.navigateTo({ url: '/pages/reader/reader?id=' + e.currentTarget.dataset.id })
  },

  // 长按：操作菜单（移出书架 / 书籍详情）
  onLong(e) {
    const id = e.currentTarget.dataset.id
    const b = B.get(id)
    wx.showActionSheet({
      itemList: ['移出书架', '书籍详情'],
      success: res => {
        if (res.tapIndex === 0) {
          S.removeShelf(id)
          this.load()
          wx.showToast({ title: '《' + b.title + '》已移出书架', icon: 'none' })
        } else {
          wx.navigateTo({ url: '/pages/detail/detail?id=' + id })
        }
      }
    })
  },

  onGoStore() {
    wx.switchTab({ url: '/pages/store/store' })
  }
})
