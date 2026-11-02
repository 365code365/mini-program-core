// 地图找房：示意地图上按区出气泡（套数 / 均价），点气泡看这个区的房源。
// 没用 <map> 组件：区级气泡是聚合数据，示意图在所有宿主上都一样能点；单套房源的真实位置在详情页「查看地图」里走 openLocation
const D = require('../../utils/data.js')
const host = require('../../utils/host.js')
const MAP_W = 702
const MAP_H = 600
Page({
  data: {
    types: [{ key: 'second', text: '二手房' }, { key: 'new', text: '新房' }, { key: 'rent', text: '租房' }],
    type: 'second',
    bubbles: [],
    district: '',
    houses: [],
    located: false,
    typeText: '二手房'
  },
  onLoad(q) {
    if (D.PRICE[q.type]) this.setData({ type: q.type })
    this.refresh()
  },
  refresh() {
    const { type, district } = this.data
    const bubbles = D.DISTRICTS.map(d => {
      const n = D.all().filter(h => h.type === type && h.district === d.name).length
      return {
        name: d.name, n,
        avg: (d.avg / 10000).toFixed(1) + '万',
        left: Math.round(d.x / 100 * MAP_W), top: Math.round(d.y / 100 * MAP_H)
      }
    })
    this.setData({
      bubbles,
      typeText: D.TYPE_TEXT[type],
      houses: district ? D.all().filter(h => h.type === type && h.district === district) : []
    })
  },
  onType(e) {
    this.setData({ type: e.currentTarget.dataset.key })
    this.refresh()
  },
  onBubble(e) {
    const name = e.currentTarget.dataset.name
    this.setData({ district: this.data.district === name ? '' : name })
    this.refresh()
  },
  // 定位到最近的区；宿主没有定位能力时退回默认的滨江（门店最多的区）
  onLocate() {
    host.locate().then(p => {
      if (!p) return
      let name = '滨江'
      if (!p.fallback) {
        let best = Infinity
        D.all().forEach(h => {
          const d = Math.pow(h.lat - p.lat, 2) + Math.pow(h.lng - p.lng, 2)
          if (d < best) { best = d; name = h.district }
        })
      } else {
        wx.showToast({ title: '暂时无法定位，先看看滨江', icon: 'none' })
      }
      this.setData({ district: name, located: true })
      this.refresh()
    })
  },
  onAll() {
    wx.navigateTo({ url: '/pages/list/list?type=' + this.data.type + '&district=' + encodeURIComponent(this.data.district) })
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  }
})
