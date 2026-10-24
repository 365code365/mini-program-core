const D = require('../../utils/data.js')

Page({
  data: {
    no: '',
    detected: null,
    entries: [
      { text: '寄快递', glyph: '寄', bg: 'linear-gradient(135deg, #ffa95c, #ff6a1a)' },
      { text: '扫一扫', glyph: '扫', bg: 'linear-gradient(135deg, #6aa1ff, #2f6bff)' },
      { text: '运费时效', glyph: '¥', bg: 'linear-gradient(135deg, #4fd6a2, #19a974)' },
      { text: '附近网点', glyph: '点', bg: 'linear-gradient(135deg, #b18cff, #7c4dff)' }
    ],
    filters: [
      { key: 'all', text: '全部' },
      { key: 'deliver', text: '派送中' },
      { key: 'transit', text: '运输中' },
      { key: 'pending', text: '待揽收' },
      { key: 'signed', text: '已签收' }
    ],
    filter: 'all',
    shown: [],
    total: 0
  },

  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 0 })
    this.refresh()
  },

  refresh() {
    const list = D.loadList().map(D.decorate)
    const f = this.data.filter
    this.setData({
      total: list.length,
      shown: f === 'all' ? list : list.filter(i => i.status === f)
    })
  },

  onInput(e) {
    const no = e.detail.value
    const key = D.detectCarrier(no)
    this.setData({ no, detected: key ? D.CARRIERS[key] : null })
  },

  onQuery() {
    const no = (this.data.no || '').trim().toUpperCase()
    if (no.length < 8) {
      wx.showToast({ title: '请输入正确的快递单号', icon: 'none' })
      return
    }
    const list = D.loadList()
    if (!list.some(i => i.no === no)) {
      list.unshift(D.mockTrack(no, D.detectCarrier(no)))
      D.saveList(list)
    }
    this.setData({ no: '', detected: null })
    wx.navigateTo({ url: '/pages/detail/detail?no=' + no })
  },

  onFilter(e) {
    this.setData({ filter: e.currentTarget.dataset.key })
    this.refresh()
  },

  onOpen(e) {
    wx.navigateTo({ url: '/pages/detail/detail?no=' + e.currentTarget.dataset.no })
  },

  onEntry(e) {
    const i = Number(e.currentTarget.dataset.i)
    if (i === 0) {
      wx.switchTab({ url: '/pages/send/send' })
    } else if (i === 1) {
      // 演示：扫码直接填入一张面单
      const no = 'SF1609283374410'
      this.setData({ no, detected: D.CARRIERS.sf })
      wx.showToast({ title: '已识别面单', icon: 'success' })
    } else if (i === 2) {
      wx.showModal({
        title: '运费时效',
        content: '杭州 → 上海\n顺丰标快 ¥18 起，次日达\n中通 ¥8 起，1-2 天\n京东特快 ¥16 起，次日达',
        showCancel: false,
        confirmText: '知道了'
      })
    } else {
      wx.showActionSheet({
        itemList: ['顺丰 文三路营业点 0.6km', '中通 学院路网点 1.1km', '菜鸟驿站 文二西路店 0.3km'],
        success: res => wx.showToast({ title: '已为你导航', icon: 'none' })
      })
    }
  },

  onPullDownRefresh() {
    this.refresh()
    wx.stopPullDownRefresh()
  }
})
