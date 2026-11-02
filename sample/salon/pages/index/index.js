const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

Page({
  data: {
    store: data.store,
    works: [],
    hot: [],
    stylists: data.stylists,
    swiperCur: 0,
    upcoming: null,
    level: ''
  },
  onLoad() {
    const works = data.works.map(w => Object.assign({}, w, { stylistName: data.stylist(w.by).name }));
    const hot = data.services.filter(s => s.hot).map(s => Object.assign({}, s, { cover: data.catCover[s.cat], catName: data.catName(s.cat) }));
    this.setData({ works, hot });
  },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 0 });
    // 最近一单待服务的预约，放在首页顶部提醒
    const next = store.appts().filter(a => a.status === 'upcoming')
      .sort((a, b) => (a.date + a.time < b.date + b.time ? -1 : 1))[0] || null;
    this.setData({ upcoming: next, level: store.levelOf(store.member().growth).cur.name });
  },
  onSwiper(e) { this.setData({ swiperCur: e.detail.current }); },
  previewWork(e) {
    const i = Number(e.currentTarget.dataset.i);
    wx.previewImage({ current: this.data.works[i].src, urls: this.data.works.map(w => w.src) });
  },
  callStore() {
    const phone = this.data.store.phone;
    wx.makePhoneCall({
      phoneNumber: phone.replace(/-/g, ''),
      fail: () => {
        wx.setClipboardData({ data: phone, success: () => wx.showToast({ title: '门店电话已复制', icon: 'none' }) });
      }
    });
  },
  navStore() {
    const s = this.data.store;
    wx.openLocation({
      latitude: s.latitude, longitude: s.longitude, name: s.name, address: s.address,
      fail: () => {
        wx.setClipboardData({ data: s.address, success: () => wx.showToast({ title: '地址已复制，可粘贴到地图导航', icon: 'none' }) });
      }
    });
  },
  goBooking() { wx.navigateTo({ url: '/pages/booking/booking' }); },
  goServices() { wx.switchTab({ url: '/pages/services/services' }); },
  goStylists() { wx.switchTab({ url: '/pages/stylists/stylists' }); },
  goMember() { wx.navigateTo({ url: '/pages/member/member' }); },
  goAppts() { wx.navigateTo({ url: '/pages/appointments/appointments' }); },
  goService(e) { wx.navigateTo({ url: '/pages/service/service?id=' + e.currentTarget.dataset.id }); },
  goStylist(e) { wx.navigateTo({ url: '/pages/stylist/stylist?id=' + e.currentTarget.dataset.id }); },
  onShareAppMessage() { return { title: '型格美业 · 在线预约发型师', path: '/pages/index/index' }; }
});
