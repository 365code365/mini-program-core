// 房源订阅：按类型 / 区域 / 户型 / 价格订阅，列出每个订阅当前匹配几套，点进去就是带好条件的列表
const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
const TYPES = ['second', 'new', 'rent']
Page({
  data: {
    subs: [],
    typeNames: TYPES.map(t => D.TYPE_TEXT[t]),
    tIndex: 0,
    districts: D.DISTRICT_NAMES,
    dIndex: 0,
    layouts: D.LAYOUTS,
    lIndex: 0,
    prices: D.PRICE.second.map(p => p.text),
    pIndex: 0,
    preview: 0
  },
  onShow() {
    this.refresh()
  },
  cond() {
    const d = this.data
    return { type: TYPES[d.tIndex], district: d.districts[d.dIndex], lIndex: d.lIndex, pIndex: d.pIndex }
  },
  refresh() {
    this.setData({ subs: S.getSubs(), preview: D.filter(this.cond()).length })
  },
  onType(e) {
    const tIndex = Number(e.detail.value)
    // 价格档跟着类型换（二手按总价、新房按单价、租房按月租）
    this.setData({ tIndex, pIndex: 0, prices: D.PRICE[TYPES[tIndex]].map(p => p.text) })
    this.refresh()
  },
  onPick(e) {
    this.setData({ [e.currentTarget.dataset.k]: Number(e.detail.value) })
    this.refresh()
  },
  onAdd() {
    const s = S.addSub(this.cond())
    if (!s) return wx.showToast({ title: '这个条件已经订阅过了', icon: 'none' })
    wx.showToast({ title: '订阅成功', icon: 'success' })
    this.setData({ dIndex: 0, lIndex: 0, pIndex: 0 })
    this.refresh()
  },
  onOpen(e) {
    const s = this.data.subs.find(x => String(x.id) === String(e.currentTarget.dataset.id))
    wx.navigateTo({ url: '/pages/list/list?type=' + s.type + '&district=' + encodeURIComponent(s.district) + '&l=' + s.lIndex + '&p=' + s.pIndex })
  },
  onRemove(e) {
    const id = e.currentTarget.dataset.id
    wx.showModal({
      title: '取消订阅',
      content: '取消后不再推送这个条件的新房源和降价提醒。',
      confirmText: '取消订阅',
      cancelText: '保留',
      confirmColor: '#f2453d',
      success: r => {
        if (!r.confirm) return
        S.removeSub(id)
        this.refresh()
      }
    })
  }
})
