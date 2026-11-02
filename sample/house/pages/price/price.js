const D = require('../../utils/data.js')
function fmt(n) {
  return String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ',')
}
Page({
  data: { districts: [], communities: [], kw: '', sortKey: 'avg' },
  onLoad() {
    this.refresh()
  },
  refresh() {
    const k = this.data.sortKey
    const districts = D.DISTRICTS.slice()
      .sort((a, b) => (k === 'mom' ? b.mom - a.mom : k === 'deals' ? b.deals - a.deals : b.avg - a.avg))
      .map(d => Object.assign({}, d, {
        avgText: fmt(d.avg), momText: (d.mom > 0 ? '+' : '') + d.mom.toFixed(1) + '%', up: d.mom > 0,
        count: D.all().filter(h => h.district === d.name).length
      }))
    const kw = this.data.kw.trim()
    // 有单价的房源（二手 / 新房）按小区列出来
    const communities = D.all().filter(h => h.unit && (!kw || (h.community + h.district).indexOf(kw) >= 0))
      .map(h => ({ id: h.id, community: h.community, district: h.district, typeText: h.typeText, unitText: fmt(h.unit) }))
    this.setData({ districts, communities })
  },
  onSort(e) {
    this.setData({ sortKey: e.currentTarget.dataset.k })
    this.refresh()
  },
  onKw(e) {
    this.setData({ kw: e.detail.value })
    this.refresh()
  },
  onDistrict(e) {
    wx.navigateTo({ url: '/pages/list/list?type=second&district=' + encodeURIComponent(e.currentTarget.dataset.name) })
  },
  onCommunity(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  },
  onSell() {
    wx.navigateTo({ url: '/pages/sell/sell' })
  }
})
