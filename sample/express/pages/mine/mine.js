const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
const SERVICE = '400-000-0100'
Page({
  data: {
    stats: [],
    coupon: null,
    couponCount: 0,
    menus: [
      { text: '地址簿', glyph: '址', bg: '#2f6bff', hint: '' },
      { text: '寄件记录', glyph: '寄', bg: '#ff6a1a', hint: '' },
      { text: '优惠券', glyph: '券', bg: '#f5a300', hint: '' },
      { text: '实名认证', glyph: '证', bg: '#19a974', hint: '已认证' },
      { text: '在线客服', glyph: '服', bg: '#7c4dff', hint: '9:00-21:00' },
      { text: '关于我们', glyph: '关', bg: '#8a919f', hint: '' }
    ]
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 })
    const list = S.getList()
    const orders = S.getOrders()
    const valid = S.getCoupons().filter(c => c.status === 'valid')
    const addrCount = S.getAddrs().length
    const going = orders.filter(o => S.stageOf(o).idx >= 0 && S.stageOf(o).idx < 3).length
    this.setData({
      stats: [
        { key: 'all', num: list.length, label: '查询' },
        { key: 'transit', num: list.filter(i => i.status === 'deliver' || i.status === 'transit').length, label: '在途' },
        { key: 'sent', num: orders.length, label: '寄件' },
        { key: 'addr', num: addrCount, label: '地址' }
      ],
      coupon: valid[0] || null,
      couponCount: valid.length,
      'menus[0].hint': addrCount + ' 个',
      'menus[1].hint': going ? going + ' 单进行中' : '',
      'menus[2].hint': valid.length + ' 张可用'
    })
  },
  onStat(e) {
    const k = String(e.currentTarget.dataset.k)
    if (k === 'sent') return this.goOrders()
    if (k === 'addr') return wx.navigateTo({ url: '/pages/address/address' })
    getApp().globalData.filter = k === 'transit' ? 'transit' : 'all'
    wx.switchTab({ url: '/pages/index/index' })
  },
  goOrders() { wx.navigateTo({ url: '/pages/orders/orders' }) },
  onMenu(e) {
    const i = Number(e.currentTarget.dataset.i)
    if (i === 0) wx.navigateTo({ url: '/pages/address/address' })
    else if (i === 1) this.goOrders()
    else if (i === 2) wx.navigateTo({ url: '/pages/coupons/coupons' })
    else if (i === 3) {
      wx.showModal({ title: '实名认证', content: '已于 2024-05-12 完成实名认证\n姓名：林*鹿\n证件：3301**********2016\n寄件时按国家规定核验寄件人身份', showCancel: false, confirmText: '知道了' })
    } else if (i === 4) {
      wx.showModal({
        title: '在线客服', content: '客服热线 ' + SERVICE + '\n服务时间 9:00-21:00', confirmText: '拨打', confirmColor: '#2f6bff',
        success: r => { if (r.confirm) host.call(SERVICE, '客服热线') }
      })
    } else {
      wx.showModal({ title: '快递100查', content: '一个单号查遍全网物流，上门取件寄全国。\n当前版本 v3.8.2', showCancel: false })
    }
  },
  onUseCoupon() {
    wx.navigateTo({ url: '/pages/coupons/coupons' })
  },
  // 设置：两个开关，选了就切换并存下来
  onSetting() {
    const s = S.settings()
    wx.showActionSheet({
      itemList: ['物流推送：' + (s.push ? '已开启' : '已关闭'), '隐私面单：' + (s.privacy ? '已开启' : '已关闭')],
      success: r => {
        const k = r.tapIndex === 0 ? 'push' : 'privacy'
        const on = !s[k]
        S.setSetting(k, on)
        wx.showToast({ title: (k === 'push' ? '物流推送' : '隐私面单（面单隐藏手机号中间四位）') + (on ? '已开启' : '已关闭'), icon: 'none' })
      }
    })
  },
  onClear() {
    wx.showModal({
      title: '清除查询记录',
      content: '将清空「我的快递」与查询历史（寄件订单不受影响），确定吗？',
      confirmColor: '#f2453d',
      success: res => {
        if (!res.confirm) return
        S.saveList([])
        S.clearHistory()
        wx.showToast({ title: '已清除', icon: 'success' })
        this.onShow()
      }
    })
  }
})
