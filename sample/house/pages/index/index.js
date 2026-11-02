const D = require('../../utils/data.js')

Page({
  data: {
    cities: ['杭州', '上海', '南京', '苏州', '宁波'],
    cityIndex: 0,
    kw: '',
    types: [
      { key: 'second', text: '二手房' },
      { key: 'new', text: '新房' },
      { key: 'rent', text: '租房' }
    ],
    type: 'second',
    typeText: '二手房',
    banners: [
      { title: '金秋看房季', sub: '预约带看送 200 元京东卡，限时 10 月', btn: '立即预约', bg: 'linear-gradient(120deg, #ff9a3d 0%, #ff5a1f 100%)' },
      { title: '未来科技城 · 新盘直击', sub: '万科、绿城 12 盘在售，最低 3.2 万/㎡', btn: '查看楼盘', bg: 'linear-gradient(120deg, #4f8bff 0%, #2f5fe0 100%)' },
      { title: '租房 0 中介费', sub: '真房源 · 假一赔百 · 7 天无理由换房', btn: '去租房', bg: 'linear-gradient(120deg, #36c89a 0%, #12976f 100%)' }
    ],
    entries: [
      { text: '二手房', glyph: '二', bg: 'linear-gradient(135deg, #ffa95c, #ff6a1f)', type: 'second' },
      { text: '新房', glyph: '新', bg: 'linear-gradient(135deg, #6aa1ff, #2f6bff)', type: 'new' },
      { text: '租房', glyph: '租', bg: 'linear-gradient(135deg, #4fd6a2, #19a974)', type: 'rent' },
      { text: '地图找房', glyph: '图', bg: 'linear-gradient(135deg, #b18cff, #7c4dff)', url: '/pages/map/map' },
      { text: '房贷计算', glyph: '算', bg: 'linear-gradient(135deg, #ff8fa3, #f2456b)', url: '/pages/calc/calc' },
      { text: '查房价', glyph: '价', bg: 'linear-gradient(135deg, #ffd35c, #f5a300)', url: '/pages/price/price' },
      { text: '卖房估价', glyph: '估', bg: 'linear-gradient(135deg, #5fd3e6, #1a9fc0)', url: '/pages/sell/sell' },
      { text: '我的预约', glyph: '约', bg: 'linear-gradient(135deg, #9aa7bd, #5f6b80)', url: '/pages/bookings/bookings' }
    ],
    houses: []
  },

  onLoad() {
    this.refresh()
  },

  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 })
  },

  refresh() {
    const t = this.data.type
    this.setData({
      houses: D.all().filter(h => h.type === t),
      typeText: D.TYPE_TEXT[t]
    })
  },

  onCity(e) {
    // 房源只覆盖杭州：选了别的城市就说明一下、留在杭州（标题与列表保持一致）
    const i = Number(e.detail.value)
    if (i !== 0) {
      wx.showModal({ title: this.data.cities[i], content: '安居找房目前只有杭州的房源，其他城市的经纪门店还在入驻。', showCancel: false, confirmText: '看杭州', confirmColor: '#ff5a1f' })
    }
    this.setData({ cityIndex: 0 })
  },

  onKw(e) {
    this.setData({ kw: e.detail.value })
  },

  onSearch() {
    const kw = (this.data.kw || '').trim()
    wx.navigateTo({ url: '/pages/list/list?type=' + this.data.type + '&kw=' + encodeURIComponent(kw) })
  },

  onType(e) {
    this.setData({ type: e.currentTarget.dataset.key })
    this.refresh()
  },

  onBanner(e) {
    const i = Number(e.currentTarget.dataset.i)
    const type = ['second', 'new', 'rent'][i]
    wx.navigateTo({ url: '/pages/list/list?type=' + type })
  },

  onEntry(e) {
    const it = this.data.entries[Number(e.currentTarget.dataset.i)]
    if (it.type) wx.navigateTo({ url: '/pages/list/list?type=' + it.type })
    else wx.navigateTo({ url: it.url })
  },
  onMarket() {
    wx.navigateTo({ url: '/pages/price/price' })
  },
  onMore() {
    wx.navigateTo({ url: '/pages/list/list?type=' + this.data.type })
  },

  onOpen(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  }
})
