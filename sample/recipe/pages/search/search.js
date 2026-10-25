const R = require('../../utils/recipes.js')
const S = require('../../utils/store.js')
Page({
  data: {
    keyword: '',
    searched: '',
    history: [],
    hot: R.HOT_TAGS,
    results: []
  },
  onShow() {
    this.setData({ history: S.history() })
  },
  onInput(e) {
    const keyword = e.detail.value
    // 清空输入框：回到「历史 + 热门」
    this.setData(keyword.trim() ? { keyword } : { keyword, searched: '', results: [] })
  },
  onConfirm() {
    this.run(this.data.keyword)
  },
  onTag(e) {
    const kw = e.currentTarget.dataset.kw
    this.setData({ keyword: kw })
    this.run(kw)
  },
  run(kw) {
    kw = (kw || '').trim()
    if (!kw) {
      wx.showToast({ title: '请输入关键词', icon: 'none' })
      return
    }
    const results = R.search(kw).map(r => ({ id: r.id, name: r.name, minutes: r.minutes, level: r.level, catText: r.catText, c1: r.c1, c2: r.c2, initial: r.initial }))
    this.setData({ searched: kw, results, history: S.addHistory(kw) })
  },
  onClear() {
    this.setData({ keyword: '', searched: '', results: [] })
  },
  onClearHistory() {
    wx.showModal({
      title: '清空搜索历史',
      content: '确定清空全部搜索历史吗？',
      success: res => {
        if (!res.confirm) return
        S.clearHistory()
        this.setData({ history: [] })
      }
    })
  },
  onRecipe(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  }
})
