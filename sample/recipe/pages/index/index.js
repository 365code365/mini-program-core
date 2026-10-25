const R = require('../../utils/recipes.js')
const PAGE_SIZE = 6
Page({
  data: {
    cats: R.CATS,
    cat: 0,
    list: [],
    total: 0,
    page: 0,
    loading: false,
    noMore: false,
    hint: ''
  },
  onLoad() {
    this.setData({ hint: getApp().globalData.hint })
    this.reload()
  },
  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 0 })
  },
  reload() {
    const r = R.page(R.CATS[this.data.cat].key, 0, PAGE_SIZE)
    this.setData({ list: r.list.map(card), total: r.total, page: 0, noMore: r.list.length >= r.total, loading: false })
  },
  // 触底分页：模拟 300ms 的加载，追加下一页（loading 期间再触底不重复加载）
  onReachBottom() {
    if (this.data.loading || this.data.noMore) return
    this.setData({ loading: true })
    const cat = this.data.cat
    setTimeout(() => {
      if (cat !== this.data.cat) return
      const p = this.data.page + 1
      const r = R.page(R.CATS[cat].key, p, PAGE_SIZE)
      const list = this.data.list.concat(r.list.map(card))
      this.setData({ list, page: p, loading: false, noMore: list.length >= r.total })
    }, 300)
  },
  onCat(e) {
    const i = Number(e.currentTarget.dataset.index)
    if (i === this.data.cat) return
    this.setData({ cat: i })
    this.reload()
    wx.pageScrollTo({ scrollTop: 0, duration: 0 })
  },
  onSearch() {
    wx.navigateTo({ url: '/pages/search/search' })
  },
  onRecipe(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  }
})
// 卡片只要展示用的字段
function card(r) {
  return { id: r.id, name: r.name, minutes: r.minutes, level: r.level, kcal: r.kcal, catText: r.catText, tags: r.tags.slice(0, 3), c1: r.c1, c2: r.c2, initial: r.initial, steps: r.steps.length }
}
