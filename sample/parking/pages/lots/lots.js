const { LOTS } = require('../../utils/data.js')
const S = require('../../utils/store.js')

Page({
  data: { sort: 'dist', onlyFree: false, keyword: '', list: [], total: LOTS.length },
  onShow() {
    this.refresh()
  },
  refresh() {
    const { sort, onlyFree } = this.data
    const kw = this.data.keyword.trim()
    const free = S.freeMap()
    let list = LOTS.map(l => Object.assign({}, l, { free: Number(free[l.id]) }))
    if (kw) list = list.filter(l => l.name.indexOf(kw) >= 0 || l.addr.indexOf(kw) >= 0)
    if (onlyFree) list = list.filter(l => l.free > 0)
    // 价格相同按距离；距离相同按价格
    list.sort((a, b) => (sort === 'price' ? a.first - b.first || a.dist - b.dist : a.dist - b.dist || a.first - b.first))
    this.setData({
      list: list.map(l => ({
        id: l.id, name: l.name, addr: l.addr, free: l.free, total: l.total,
        distText: l.dist < 1 ? Math.round(l.dist * 1000) + 'm' : l.dist.toFixed(1) + 'km',
        priceText: '首小时 ¥' + l.first,
        level: l.free === 0 ? 'full' : l.free / l.total < 0.1 ? 'tight' : 'ok'
      }))
    })
  },
  onSort(e) {
    this.setData({ sort: e.currentTarget.dataset.s })
    this.refresh()
  },
  onOnlyFree(e) {
    this.setData({ onlyFree: e.detail.value })
    this.refresh()
  },
  onSearch(e) {
    this.setData({ keyword: e.detail.value })
    this.refresh()
  },
  clearSearch() {
    this.setData({ keyword: '' })
    this.refresh()
  },
  onLot(e) {
    wx.navigateTo({ url: '/pages/lot/lot?id=' + e.currentTarget.dataset.id })
  }
})
