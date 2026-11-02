const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
Page({
  data: {
    no: '',
    noSeq: 0,
    focus: false,
    detected: null,
    history: [],
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
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 })
    // 「我的 → 在途」切过来时带着筛选（switchTab 不能带参数）
    const app = getApp()
    if (app.globalData.filter) {
      this.setData({ filter: app.globalData.filter })
      app.globalData.filter = ''
    }
    this.refresh()
  },
  refresh() {
    const list = S.getList().map(D.decorate)
    const f = this.data.filter
    this.setData({
      total: list.length,
      history: S.history(),
      shown: f === 'all' ? list : list.filter(i => i.status === f)
    })
  },
  onInput(e) {
    const no = e.detail.value
    const key = D.detectCarrier(no)
    this.setData({ no, detected: key ? D.CARRIERS[key] : null })
  },
  onBlur() { this.setData({ focus: false }) },
  query(raw) {
    const no = String(raw || '').trim().toUpperCase()
    if (!/^[0-9A-Z]{8,20}$/.test(no)) {
      wx.showToast({ title: '请输入正确的快递单号', icon: 'none' })
      return
    }
    S.query(no)
    this.setData({ no: '', noSeq: this.data.noSeq + 1, detected: null })
    wx.navigateTo({ url: '/pages/detail/detail?no=' + no })
  },
  onQuery() { this.query(this.data.no) },
  onHistory(e) { this.query(e.currentTarget.dataset.no) },
  onClearHistory() {
    S.clearHistory()
    this.setData({ history: [] })
  },
  onFilter(e) {
    this.setData({ filter: String(e.currentTarget.dataset.key) })
    this.refresh()
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/detail/detail?no=' + e.currentTarget.dataset.no })
  },
  onEntry(e) {
    const i = Number(e.currentTarget.dataset.i)
    if (i === 0) this.goSend()
    else if (i === 1) this.scan()
    else if (i === 2) this.prices()
    else this.outlets()
  },
  goSend() { wx.switchTab({ url: '/pages/send/send' }) },
  // 扫面单条码；宿主不能扫码时把光标放进输入框，让用户手动输入
  scan() {
    host.scan().then(no => {
      if (no === '') return
      if (no === null) {
        this.setData({ focus: true })
        wx.showToast({ title: '当前环境无法打开摄像头，请手动输入单号', icon: 'none' })
        return
      }
      const key = D.detectCarrier(no)
      this.setData({ no, noSeq: this.data.noSeq + 1, detected: key ? D.CARRIERS[key] : null })
    })
  },
  // 运费时效：从默认寄件地址出发，各快递 1kg 的首重价与时效
  prices() {
    const a = S.defaultAddr()
    const from = a ? S.cityOf(a) : '杭州'
    wx.showModal({
      title: '运费时效（' + from + '出发 · 1kg）',
      content: D.PRODUCTS.map(p => p.name + ' ¥' + p.first + ' 起，' + p.eta + '，续重 ¥' + p.extra + '/kg').join('\n'),
      confirmText: '去寄件',
      cancelText: '关闭',
      confirmColor: '#2f6bff',
      success: r => { if (r.confirm) this.goSend() }
    })
  },
  outlets() {
    wx.showActionSheet({
      itemList: D.OUTLETS.map(o => o.name + ' ' + o.dist),
      success: r => host.openMap(D.OUTLETS[r.tapIndex])
    })
  },
  onPullDownRefresh() {
    this.refresh()
    wx.stopPullDownRefresh()
  }
})
