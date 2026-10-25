const S = require('../../utils/store.js')

function fmt(sec) {
  if (sec < 60) return sec + ' 秒'
  const m = Math.floor(sec / 60)
  if (m < 60) return m + ' 分钟'
  return Math.floor(m / 60) + ' 小时 ' + (m % 60) + ' 分'
}

Page({
  data: {
    totalSeconds: 0,
    todaySeconds: 0,
    totalText: '0 秒',
    todayText: '0 秒',
    days: 0,
    shelfCount: 0,
    finished: 0,
    bookmarks: [],
    settings: null
  },

  onShow() {
    const st = S.loadStats()
    const prog = S.loadProgress()
    const shelf = S.loadShelf()
    this.setData({
      totalSeconds: st.total,
      todaySeconds: st.today,
      totalText: fmt(st.total),
      todayText: fmt(st.today),
      days: st.days,
      shelfCount: shelf.length,
      finished: Object.keys(prog).filter(k => prog[k].pct >= 100).length,
      bookmarks: S.loadBookmarks(),
      settings: S.loadSettings()
    })
  },

  // 点书签：打开对应书的那一章、滚到加书签时的位置
  onMark(e) {
    const m = this.data.bookmarks[Number(e.currentTarget.dataset.i)]
    wx.navigateTo({ url: '/pages/reader/reader?id=' + m.bookId + '&chapter=' + m.chapter + '&top=' + m.top })
  },

  onDelMark(e) {
    const m = this.data.bookmarks[Number(e.currentTarget.dataset.i)]
    wx.showModal({
      title: '删除书签',
      content: '《' + m.bookTitle + '》' + m.chapterTitle,
      success: r => {
        if (!r.confirm) return
        S.removeBookmark(m.id)
        this.onShow()
      }
    })
  },

  onReset() {
    wx.showModal({
      title: '清空阅读记录',
      content: '阅读时长与书签都会被清空，书架保留',
      success: r => {
        if (!r.confirm) return
        wx.removeStorageSync('rd_stats')
        wx.removeStorageSync('rd_bookmarks')
        this.onShow()
      }
    })
  }
})
