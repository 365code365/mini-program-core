const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
const LABELS = ['实景', '户型', '卧室']
Page({
  data: {
    h: null,
    agent: null,
    imgs: [],
    cur: 0,
    facts: [],
    faved: false,
    bookedCount: 0,
    pending: null,
    monthly: 0
  },
  onLoad(q) {
    const h = D.get(q.id || 'h1')
    this.id = h.id
    const facts = [
      { k: '朝向', v: h.face },
      { k: '楼层', v: h.floor },
      { k: '装修', v: h.deco },
      { k: '电梯', v: h.lift },
      { k: h.type === 'new' ? '交付' : '年代', v: h.year + ' 年' },
      { k: '供暖', v: h.heat },
      { k: '小区', v: h.community },
      { k: '区域', v: h.district }
    ]
    if (h.type === 'second') facts.splice(4, 0, { k: '单价', v: h.unit + ' 元/㎡' })
    // 参考月供：三成首付、商贷 30 年、3.5% 等额本息（租房没有）
    let monthly = 0
    if (h.type !== 'rent') {
      const loan = h.price * 10000 * 0.7
      const r = 0.035 / 12
      const n = 360
      monthly = Math.round(loan * r * Math.pow(1 + r, n) / (Math.pow(1 + r, n) - 1))
    }
    this.setData({
      h, facts, monthly,
      agent: D.AGENTS[h.agent],
      imgs: h.images.map((n, i) => ({ src: '/images/' + n + '.png', label: LABELS[i] }))
    })
    wx.setNavigationBarTitle({ title: h.community })
    S.pushHistory(h.id)
  },
  onShow() {
    const booked = S.getBookings().filter(b => b.houseId === this.id && b.status !== 'cancelled').length
    this.setData({ faved: S.isFav(this.id), bookedCount: 23 + booked, pending: S.pendingFor(this.id) })
  },
  onSwiper(e) {
    this.setData({ cur: e.detail.current })
  },
  onGTab(e) {
    this.setData({ cur: Number(e.currentTarget.dataset.i) })
  },
  onPreview(e) {
    wx.previewImage({
      current: this.data.imgs[Number(e.currentTarget.dataset.i)].src,
      urls: this.data.imgs.map(i => i.src)
    })
  },
  onFav() {
    const on = S.toggleFav(this.id)
    this.setData({ faved: on })
    wx.showToast({ title: on ? '已加入收藏' : '已取消收藏', icon: 'none' })
  },
  onShare() {
    const h = this.data.h
    wx.showActionSheet({
      itemList: ['复制房源链接', '复制房源信息'],
      success: res => {
        if (res.tapIndex === 0) host.copy('https://anju.example.com/house/' + this.id, '链接已复制，去粘贴给朋友吧')
        else host.copy(h.title + '\n' + h.brief + '\n' + h.priceMain + h.priceUnit + '\n' + h.addr, '房源信息已复制')
      }
    })
  },
  onMap() {
    const h = this.data.h
    host.openMap({ lat: h.lat, lng: h.lng, name: h.community, addr: '杭州市' + h.addr, extra: h.subway })
  },
  onCalc() {
    wx.navigateTo({ url: '/pages/calc/calc?price=' + this.data.h.price })
  },
  onChat() {
    wx.showModal({
      title: this.data.agent.name,
      content: '您好，这套 ' + this.data.h.community + ' 目前可以看房，周末也能约，您想哪天来？',
      confirmText: '去预约',
      confirmColor: '#ff5a1f',
      success: r => r.confirm && this.onBook()
    })
  },
  onCall() {
    host.call(this.data.agent.phone, '经纪人 ' + this.data.agent.name)
  },
  onBook() {
    // 这套已经约过还没去看：去预约记录（那里能改约 / 取消），别再约一次
    if (this.data.pending) return wx.navigateTo({ url: '/pages/bookings/bookings' })
    wx.navigateTo({ url: '/pages/booking/booking?id=' + this.id })
  }
})
