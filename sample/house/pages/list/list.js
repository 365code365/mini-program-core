const D = require('../../utils/data.js')

const S = require('../../utils/store.js')
Page({
  data: {
    types: [{ key: 'second', text: '二手' }, { key: 'new', text: '新房' }, { key: 'rent', text: '租房' }],
    type: 'second',
    typeText: '二手房',
    kw: '',
    districts: D.DISTRICT_NAMES,
    dIndex: 0,
    priceRanges: D.PRICE,
    pIndex: 0,
    layouts: D.LAYOUTS,
    lIndex: 0,
    sorts: ['默认', '总价低', '面积大', '最新'],
    sort: 0,
    houses: []
  },

  // query：type / kw / district（区名）/ l（户型下标）/ p（价格下标）—— 首页、查房价、地图找房、订阅都从这里进
  onLoad(q) {
    const type = D.PRICE[q.type] ? q.type : 'second'
    const district = q.district ? decodeURIComponent(q.district) : ''
    const dIndex = Math.max(0, D.DISTRICT_NAMES.indexOf(district))
    const lIndex = Math.min(D.LAYOUTS.length - 1, Math.max(0, Number(q.l) || 0))
    const pIndex = Math.min(D.PRICE[type].length - 1, Math.max(0, Number(q.p) || 0))
    this.setData({ type, kw: q.kw ? decodeURIComponent(q.kw) : '', dIndex, lIndex, pIndex })
    this.refresh()
  },

  cond() {
    const { type, kw, districts, dIndex, pIndex, lIndex } = this.data
    return { type, kw, district: districts[dIndex], pIndex, lIndex }
  },
  refresh() {
    const { type, sort } = this.data
    let list = D.filter(this.cond())
    if (sort === 1) list = list.sort((a, b) => a.price - b.price)
    if (sort === 2) list = list.sort((a, b) => b.area - a.area)
    if (sort === 3) list = list.sort((a, b) => b.year - a.year)
    this.setData({ houses: list, typeText: D.TYPE_TEXT[type] })
    wx.setNavigationBarTitle({ title: D.TYPE_TEXT[type] })
  },
  // 把当前条件存成订阅（关键词不进订阅：订阅按区域 / 户型 / 价格推新房源）
  onSubscribe() {
    const c = this.cond()
    const s = S.addSub(c)
    if (!s) return wx.showToast({ title: '这个条件已经订阅过了', icon: 'none' })
    wx.showModal({
      title: '订阅成功', content: D.condText(c) + '\n有新房源或降价会第一时间通知你', confirmText: '查看订阅', cancelText: '知道了', confirmColor: '#ff5a1f',
      success: r => r.confirm && wx.navigateTo({ url: '/pages/subscribe/subscribe' })
    })
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
