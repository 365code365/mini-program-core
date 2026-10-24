const D = require('../../utils/data.js')

// 价格档：二手/新房按总价（万），租房按月租（元）
const PRICE = {
  second: [{ text: '不限' }, { text: '400 万以下', max: 400 }, { text: '400-500 万', min: 400, max: 500 }, { text: '500 万以上', min: 500 }],
  new: [{ text: '不限' }, { text: '4 万/㎡ 以下', umax: 40000 }, { text: '4 万/㎡ 以上', umin: 40000 }],
  rent: [{ text: '不限' }, { text: '5000 元以下', max: 5000 }, { text: '5000 元以上', min: 5000 }]
}

Page({
  data: {
    types: [{ key: 'second', text: '二手' }, { key: 'new', text: '新房' }, { key: 'rent', text: '租房' }],
    type: 'second',
    typeText: '二手房',
    kw: '',
    districts: ['不限', '滨江', '西湖', '余杭', '拱墅', '上城'],
    dIndex: 0,
    priceRanges: PRICE,
    pIndex: 0,
    layouts: ['不限', '一室', '两室', '三室', '四室及以上'],
    lIndex: 0,
    sorts: ['默认', '总价低', '面积大', '最新'],
    sort: 0,
    houses: []
  },

  onLoad(q) {
    this.setData({ type: q.type || 'second', kw: q.kw ? decodeURIComponent(q.kw) : '' })
    this.refresh()
  },

  refresh() {
    const { type, kw, districts, dIndex, pIndex, lIndex, sort } = this.data
    const pr = PRICE[type][pIndex] || {}
    let list = D.all().filter(h => {
      if (h.type !== type) return false
      if (kw && (h.title + h.community + h.district + h.subway).indexOf(kw) < 0) return false
      if (dIndex && h.district !== districts[dIndex]) return false
      if (pr.min && h.price < pr.min) return false
      if (pr.max && h.price >= pr.max) return false
      if (pr.umin && h.unit < pr.umin) return false
      if (pr.umax && h.unit >= pr.umax) return false
      if (lIndex && lIndex < 4 && h.rooms !== lIndex) return false
      if (lIndex === 4 && h.rooms < 4) return false
      return true
    })
    if (sort === 1) list = list.sort((a, b) => a.price - b.price)
    if (sort === 2) list = list.sort((a, b) => b.area - a.area)
    if (sort === 3) list = list.sort((a, b) => b.year - a.year)
    this.setData({ houses: list, typeText: D.TYPE_TEXT[type] })
    wx.setNavigationBarTitle({ title: D.TYPE_TEXT[type] })
  },

  onType(e) {
    this.setData({ type: e.currentTarget.dataset.key, pIndex: 0 })
    this.refresh()
  },
  onKw(e) { this.setData({ kw: e.detail.value }); this.refresh() },
  onClearKw() { this.setData({ kw: '' }); this.refresh() },
  onDistrict(e) { this.setData({ dIndex: Number(e.detail.value) }); this.refresh() },
  onPrice(e) { this.setData({ pIndex: Number(e.detail.value) }); this.refresh() },
  onLayout(e) { this.setData({ lIndex: Number(e.detail.value) }); this.refresh() },

  onSort() {
    wx.showActionSheet({
      itemList: this.data.sorts,
      success: res => {
        this.setData({ sort: res.tapIndex })
        this.refresh()
      }
    })
  },

  onReset() {
    this.setData({ kw: '', dIndex: 0, pIndex: 0, lIndex: 0, sort: 0 })
    this.refresh()
  },

  onOpen(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  }
})
