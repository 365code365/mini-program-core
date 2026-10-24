const D = require('../../utils/data.js')

const PROGRESS = { pending: 6, transit: 55, deliver: 85, signed: 100, problem: 40 }

Page({
  data: {
    pkg: null,
    progress: 0,
    courierInitial: '',
    subscribed: false
  },

  onLoad(query) {
    this.no = query.no || 'SF1428930675512'
    this.load()
  },

  load() {
    const raw = D.loadList().find(i => i.no === this.no) || D.loadList()[0]
    const pkg = D.decorate(raw)
    const subs = wx.getStorageSync('express_subs') || []
    this.setData({
      pkg,
      progress: PROGRESS[pkg.status] || 50,
      courierInitial: pkg.courier.name.charAt(0),
      subscribed: subs.indexOf(pkg.no) >= 0
    })
    wx.setNavigationBarTitle({ title: pkg.carrierName })
  },

  onCopy() {
    wx.setClipboardData({
      data: this.data.pkg.no,
      success: () => wx.showToast({ title: '单号已复制', icon: 'success' })
    })
  },

  onCall() {
    const c = this.data.pkg.courier
    wx.showModal({
      title: '联系快递员',
      content: c.name + '  ' + c.phone,
      confirmText: '拨打',
      success: res => {
        if (res.confirm) wx.makePhoneCall({ phoneNumber: c.phone.replace(/\s/g, ''), fail() {} })
      }
    })
  },

  onRemark() {
    wx.showActionSheet({
      itemList: ['数码配件', '衣物鞋包', '生鲜食品', '文件资料'],
      success: res => {
        const names = ['数码配件', '衣物鞋包', '生鲜食品', '文件资料']
        const list = D.loadList()
        const it = list.find(i => i.no === this.no)
        if (it) {
          it.remark = names[res.tapIndex]
          D.saveList(list)
          this.load()
          wx.showToast({ title: '备注已更新', icon: 'none' })
        }
      }
    })
  },

  onSubscribe() {
    const subs = wx.getStorageSync('express_subs') || []
    const i = subs.indexOf(this.no)
    if (i >= 0) subs.splice(i, 1)
    else subs.push(this.no)
    wx.setStorageSync('express_subs', subs)
    this.setData({ subscribed: i < 0 })
    wx.showToast({ title: i < 0 ? '有新动态会通知你' : '已关闭推送', icon: 'none' })
  }
})
