const D = require('../../utils/data.js');
const F = require('../../utils/fmt.js');

Page({
  data: { v: null, photos: [], wall: [], cur: 0, sport: '', distText: '' },
  onLoad(q) {
    const v = D.getVenue(q.id) || D.VENUES[0];
    this.v = v;
    const loc = getApp().globalData.loc || D.DEFAULT_LOC;
    // 实拍墙：不同高度的图走瀑布流
    const wall = [
      { src: v.photo, h: 220, t: '主场地' }, { src: '/images/venue/p1.jpg', h: 160, t: '更衣室' },
      { src: '/images/venue/p2.jpg', h: 180, t: '休息区' }, { src: v.photo, h: 140, t: '夜场灯光' }
    ];
    this.setData({
      v, wall, photos: [v.photo, '/images/venue/p1.jpg', '/images/venue/p2.jpg'],
      sport: D.sportName(v.sport), distText: F.km(D.distance(loc, { latitude: v.lat, longitude: v.lng }))
    });
    wx.setNavigationBarTitle({ title: v.name });
  },
  onSwiper(e) { this.setData({ cur: e.detail.current }); },
  toPrice() { wx.pageScrollTo({ selector: '#price', offsetTop: -10, duration: 300 }); },
  preview(e) {
    const src = e.currentTarget.dataset.src;
    wx.previewImage({ current: src, urls: this.data.photos });
  },
  addrMore() {
    const v = this.v;
    wx.showActionSheet({
      itemList: ['导航到场馆', '复制地址'],
      success: (r) => {
        if (r.tapIndex === 0) {
          wx.openLocation({
            latitude: v.lat, longitude: v.lng, name: v.name, address: v.addr,
            fail: () => wx.setClipboardData({ data: v.addr, success: () => wx.showToast({ title: '无法打开地图，地址已复制', icon: 'none' }) })
          });
        } else wx.setClipboardData({ data: v.addr });
      }
    });
  },
  call() {
    const phone = this.v.phone;
    wx.makePhoneCall({
      phoneNumber: phone.replace('-', ''),
      // 没有通话能力（桌面 / 平板）：把号码复制给用户
      fail: () => wx.setClipboardData({ data: phone, success: () => wx.showToast({ title: '已复制场馆电话 ' + phone, icon: 'none' }) })
    });
  },
  book() { wx.navigateTo({ url: '/pages/booking/booking?id=' + this.v.id }); },
  onShareAppMessage() {
    return { title: '一起去' + this.v.name + '打' + this.data.sport + '吧', path: '/pages/venue/venue?id=' + this.v.id, imageUrl: this.v.photo };
  }
});
