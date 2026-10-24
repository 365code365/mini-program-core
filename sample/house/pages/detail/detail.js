const D = require('../../utils/data.js')

const LABELS = ['实景', '户型', '卧室']

Page({
  data: {
    h: null,
    agent: null,
    imgs: [],
    cur: 0,
    facts: [],
    faved: false,
    bookedCount: 0
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
    this.setData({
      h, facts,
      agent: D.AGENTS[h.agent],
      imgs: h.images.map((n, i) => ({ src: '/images/' + n + '.png', label: LABELS[i] }))
    })
    wx.setNavigationBarTitle({ title: h.community })
    const hist = (wx.getStorageSync('house_history') || []).filter(id => id !== h.id)
    hist.unshift(h.id)
    wx.setStorageSync('house_history', hist.slice(0, 20))
  },

  onShow() {
    const booked = D.getBookings().filter(b => b.houseId === this.id).length
    this.setData({ faved: D.getFavs().indexOf(this.id) >= 0, bookedCount: 23 + booked })
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
    const on = D.toggleFav(this.id)
    this.setData({ faved: on })
    wx.showToast({ title: on ? '已加入收藏' : '已取消收藏', icon: 'none' })
  },

  onShare() {
    wx.showActionSheet({
      itemList: ['发送给朋友', '生成房源海报', '复制房源链接'],
      success: res => {
        if (res.tapIndex === 2) wx.setClipboardData({ data: 'https://anju.example.com/house/' + this.id })
        else wx.showToast({ title: '已生成', icon: 'none' })
      }
    })
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
    wx.makePhoneCall({ phoneNumber: '4008106688', fail() {} })
  },

  onBook() {
    wx.navigateTo({ url: '/pages/booking/booking?id=' + this.id })
  }
})
