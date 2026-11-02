// 我的：会员概览、预约入口、收藏的发型师、门店电话 / 导航
const store = require('../../utils/store.js');
const data = require('../../utils/data.js');
Page({
  data: { m: null, lv: null, profile: {}, counts: { upcoming: 0, done: 0, review: 0 }, favs: [], shop: data.store, next: null },
  onShow() {
    const tb = this.getTabBar && this.getTabBar();
    if (tb) tb.setData({ selected: 3 });
    const m = store.member(), all = store.appts();
    const upcoming = all.filter(a => a.status === 'upcoming');
    this.setData({
      m, lv: store.levelOf(m.growth), profile: store.profile(),
      counts: { upcoming: upcoming.length, done: all.filter(a => a.status === 'done').length, review: all.filter(a => a.status === 'done' && !a.review).length },
      next: upcoming.slice().sort((a, b) => (a.date + a.time < b.date + b.time ? -1 : 1))[0] || null,
      favs: store.favs().map(id => data.stylist(id)).filter(Boolean)
    });
  },
  goAppts() { wx.navigateTo({ url: '/pages/appointments/appointments' }); },
  goMember() { wx.navigateTo({ url: '/pages/member/member' }); },
  goStylist(e) { wx.navigateTo({ url: '/pages/stylist/stylist?id=' + e.currentTarget.dataset.id }); },
  goStylists() { wx.switchTab({ url: '/pages/stylists/stylists' }); },
  call() {
    const phone = this.data.shop.phone;
    wx.makePhoneCall({ phoneNumber: phone, fail: () => wx.showModal({ title: '门店电话', content: phone, confirmText: '复制号码', success: r => { if (r.confirm) wx.setClipboardData({ data: phone }); } }) });
  },
  nav() {
    const s = this.data.shop;
    wx.openLocation({
      latitude: s.latitude, longitude: s.longitude, name: s.name, address: s.address, scale: 17,
      fail: () => wx.showModal({ title: '门店地址', content: s.address, confirmText: '复制地址', success: r => { if (r.confirm) wx.setClipboardData({ data: s.address }); } })
    });
  },
  editName() {
    wx.showActionSheet({
      itemList: ['清除我的到店人信息', '清除收藏'],
      success: r => {
        if (r.tapIndex === 0) store.saveProfile({ name: '', phone: '' });
        else wx.setStorageSync('salon_favs', []);
        wx.showToast({ title: '已清除', icon: 'none' });
        this.onShow();
      }
    });
  }
});
