const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
const PROGRESS = { pending: 6, transit: 55, deliver: 85, signed: 100, problem: 40 }
const REMARKS = ['数码配件', '衣物鞋包', '生鲜食品', '文件资料', '日用百货']
Page({
  data: {
    pkg: null,
    progress: 0,
    courierInitial: '',
    subscribed: false
  },
  onLoad(query) {
    this.no = String(query.no || '')
  },
  onShow() {
    this.load()
  },
  load() {
    const raw = S.findPkg(this.no)
    if (!raw) { this.setData({ pkg: null }); return }
    const pkg = D.decorate(raw)
    this.setData({
      pkg,
      progress: PROGRESS[pkg.status] || 50,
      courierInitial: pkg.courier.name.charAt(0),
      subscribed: S.subs().indexOf(pkg.no) >= 0
    })
    wx.setNavigationBarTitle({ title: pkg.carrierName })
  },
  onCopy() {
    host.copy(this.data.pkg.no, '单号已复制')
  },
  onCall() {
    const c = this.data.pkg.courier
    host.call(c.phone, '快递员 ' + c.name)
  },
  onCarrierCall() {
    const c = D.CARRIERS[this.data.pkg.carrier]
    host.call(c.phone, c.name + '客服')
  },
  onRemark() {
    wx.showActionSheet({
      itemList: REMARKS,
      success: res => {
        const list = S.getList()
        const it = list.find(i => i.no === this.no)
        if (!it) return
        it.remark = REMARKS[res.tapIndex]
        S.saveList(list)
        this.load()
        wx.showToast({ title: '备注已更新', icon: 'none' })
      }
    })
  },
  onSubscribe() {
    const on = S.toggleSub(this.no)
    this.setData({ subscribed: on })
    wx.showToast({ title: on ? '有新动态会通知你' : '已关闭推送', icon: 'none' })
  },
  onOrder() {
    wx.navigateTo({ url: '/pages/order/order?id=' + this.data.pkg.orderId })
  },
  // 删除记录：自己寄的件要先取消订单（或等签收后再删）
  onRemove() {
    const p = this.data.pkg
    if (p.orderId && p.status !== 'signed') {
      wx.showToast({ title: '寄件订单进行中，不能删除记录', icon: 'none' })
      return
    }
    wx.showModal({
      title: '删除记录',
      content: '从「我的快递」中删除 ' + p.no + '？',
      confirmText: '删除',
      confirmColor: '#f2453d',
      success: r => {
        if (!r.confirm) return
        S.removePkg(p.no)
        wx.navigateBack()
      }
    })
  },
  onHome() {
    wx.switchTab({ url: '/pages/index/index' })
  }
})
