const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
Page({
  data: {
    tab: 0,
    tabs: ['待出行', '已完成'],
    counts: [0, 0],
    list: []
  },
  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 1 })
    this.refresh()
  },
  refresh() {
    const all = S.orders().map(o => {
      const st = S.orderState(o)
      return Object.assign({}, o, { done: st.done, stateText: st.text, dateText: D.dateLabel(o.date), namesText: o.names.join('、'), seatsText: o.seats.length ? o.seats.join(' ') : '' })
    })
    const todo = all.filter(o => !o.done)
    const done = all.filter(o => o.done)
    this.setData({ counts: [todo.length, done.length], list: this.data.tab === 0 ? todo : done })
  },
  onTab(e) {
    this.setData({ tab: Number(e.currentTarget.dataset.i) })
    this.refresh()
  },
  onCancel(e) {
    const o = this.data.list[Number(e.currentTarget.dataset.index)]
    wx.showModal({
      title: '取消订单',
      content: '确定取消 ' + o.no + '（' + o.dateText + '）的 ' + o.count + ' 张车票吗？票款 ¥' + o.total + ' 将原路退回',
      confirmText: '确定取消',
      cancelText: '再想想',
      success: res => {
        if (!res.confirm) return
        S.cancelOrder(o.id)
        this.refresh()
        wx.showToast({ title: '已取消', icon: 'none' })
      }
    })
  },
  onGo() {
    wx.switchTab({ url: '/pages/index/index' })
  }
})
