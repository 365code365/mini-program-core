const D = require('../../utils/data.js')

Page({
  data: {
    stats: [],
    menus: [
      { text: '地址簿', glyph: '址', bg: '#2f6bff', hint: '' },
      { text: '寄件记录', glyph: '寄', bg: '#ff6a1a', hint: '' },
      { text: '实名认证', glyph: '证', bg: '#19a974', hint: '已认证' },
      { text: '在线客服', glyph: '服', bg: '#7c4dff', hint: '9:00-21:00' },
      { text: '关于我们', glyph: '关', bg: '#8a919f', hint: '' }
    ]
  },

  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 2 })
    const list = D.loadList()
    const sent = list.filter(i => i.remark && i.remark.indexOf('寄件') === 0).length
    const addrCount = D.loadAddresses().length
    this.setData({
      stats: [
        { num: list.length, label: '查询' },
        { num: list.filter(i => i.status === 'deliver' || i.status === 'transit').length, label: '在途' },
        { num: sent, label: '寄件' },
        { num: addrCount, label: '地址' }
      ],
      'menus[0].hint': addrCount + ' 个'
    })
  },

  onMenu(e) {
    const i = Number(e.currentTarget.dataset.i)
    if (i === 0) wx.navigateTo({ url: '/pages/address/address' })
    else if (i === 1) wx.switchTab({ url: '/pages/index/index' })
    else if (i === 3) {
      wx.showModal({ title: '在线客服', content: '客服热线 400-000-0100\n服务时间 9:00-21:00', confirmText: '拨打', success: r => r.confirm && wx.makePhoneCall({ phoneNumber: '4000000100', fail() {} }) })
    } else if (i === 4) {
      wx.showModal({ title: '快递100查', content: '一个单号查遍全网物流。\n当前版本 v3.8.2', showCancel: false })
    } else {
      wx.showToast({ title: '已完成实名认证', icon: 'none' })
    }
  },

  onUseCoupon() {
    wx.switchTab({ url: '/pages/send/send' })
  },

  onSetting() {
    wx.showActionSheet({
      itemList: ['物流推送设置', '隐私设置', '切换账号'],
      success: () => wx.showToast({ title: '设置已保存', icon: 'none' })
    })
  },

  onClear() {
    wx.showModal({
      title: '清除查询记录',
      content: '将清空「我的快递」列表并恢复示例数据，确定吗？',
      confirmColor: '#f2453d',
      success: res => {
        if (!res.confirm) return
        wx.removeStorageSync('express_list')
        wx.showToast({ title: '已清除', icon: 'success' })
        this.onShow()
      }
    })
  }
})
