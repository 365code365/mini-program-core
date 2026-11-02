const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
const TABS = [
  { key: 'all', text: '全部' },
  { key: 'pending', text: '待带看' },
  { key: 'done', text: '已带看' },
  { key: 'cancelled', text: '已取消' }
]
Page({
  data: { tabs: TABS, tab: 0, all: [], shown: [], counts: {} },
  onLoad(q) {
    const i = TABS.findIndex(t => t.key === q.tab)
    if (i >= 0) this.setData({ tab: i })
  },
  onShow() {
    this.refresh()
  },
  refresh() {
    const all = S.getBookings()
    const counts = { all: all.length }
    TABS.slice(1).forEach(t => { counts[t.key] = all.filter(b => b.status === t.key).length })
    const key = TABS[this.data.tab].key
    this.setData({ all, counts, shown: key === 'all' ? all : all.filter(b => b.status === key) })
  },
  onTab(e) {
    this.setData({ tab: Number(e.currentTarget.dataset.i) })
    this.refresh()
  },
  find(e) {
    return this.data.all.find(b => String(b.id) === String(e.currentTarget.dataset.id))
  },
  onHouse(e) {
    const b = this.find(e)
    if (!D.find(b.houseId)) return wx.showToast({ title: '这套房源已下架', icon: 'none' })
    wx.navigateTo({ url: '/pages/detail/detail?id=' + b.houseId })
  },
  onCancel(e) {
    const b = this.find(e)
    wx.showModal({
      title: '取消预约',
      content: '取消 ' + b.time + ' 去看「' + b.community + '」的预约？经纪人' + b.agent + '会收到通知。',
      confirmText: '取消预约',
      cancelText: '再想想',
      confirmColor: '#f2453d',
      success: r => {
        if (!r.confirm) return
        S.cancelBooking(b.id)
        wx.showToast({ title: '已取消预约', icon: 'none' })
        this.refresh()
      }
    })
  },
  onReschedule(e) {
    wx.navigateTo({ url: '/pages/booking/booking?rebook=' + this.find(e).id })
  },
  onAgain(e) {
    const b = this.find(e)
    if (!D.find(b.houseId)) return wx.showToast({ title: '这套房源已下架', icon: 'none' })
    if (S.pendingFor(b.houseId)) return wx.showToast({ title: '这套房已有待带看的预约', icon: 'none' })
    wx.navigateTo({ url: '/pages/booking/booking?id=' + b.houseId })
  },
  onCall(e) {
    const b = this.find(e)
    host.call(b.agentPhone, '经纪人 ' + b.agent)
  },
  onGo() {
    wx.switchTab({ url: '/pages/index/index' })
  }
})
