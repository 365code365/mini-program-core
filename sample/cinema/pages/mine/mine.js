const D = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const PREFS = ['IMAX · 中间排', '杜比 · 中间排', '2D · 后排', 'IMAX · 后排', '不限'];
const SERVICE_PHONE = '400-820-6621';
Page({
  data: { wishes: [], wishCount: 0, seen: 0, reviewCount: 0, ticketCount: 0, vip: false, balance: 0, menus: [] },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    const ids = store.wishes();
    const tickets = store.tickets();
    const coupons = store.couponLists().usable.length;
    const fav = D.getCinema(store.favCinema());
    this.setData({
      wishes: ids.map(id => D.getMovie(id)), wishCount: ids.length,
      ticketCount: tickets.length, seen: store.seenCount(), reviewCount: store.reviews().length,
      vip: store.isVip(), balance: store.card().balance,
      menus: [
        { key: 'orders', name: '全部订单', g: '单', extra: tickets.filter(t => t.status === 'unused').length + ' 张待观影', bg: 'linear-gradient(135deg, #FF9A8B, #E8243F)' },
        { key: 'coupons', name: '优惠券', g: '券', extra: coupons + ' 张可用', bg: 'linear-gradient(135deg, #FFD36B, #F08A24)' },
        { key: 'card', name: '会员卡', g: '卡', extra: '余额 ¥' + store.card().balance, bg: 'linear-gradient(135deg, #C7B2FF, #6A4BD8)' },
        { key: 'cinema', name: '常去影院', g: '院', extra: fav.name.replace(/（.*）/, ''), bg: 'linear-gradient(135deg, #8EC5FF, #3D7BFF)' },
        { key: 'pref', name: '观影偏好', g: '好', extra: store.pref(), bg: 'linear-gradient(135deg, #FFB3D1, #D9407A)' },
        { key: 'service', name: '联系客服', g: '服', extra: SERVICE_PHONE, bg: 'linear-gradient(135deg, #8DE0B9, #1E9E6A)' }
      ]
    });
  },
  onVip() { wx.navigateTo({ url: '/pages/card/card' }); },
  onTickets() { wx.switchTab({ url: '/pages/tickets/tickets' }); },
  onSeen() {
    wx.setStorageSync(store.K.ticketTab, 'used');
    wx.switchTab({ url: '/pages/tickets/tickets' });
  },
  onReviews() { wx.navigateTo({ url: '/pages/reviews/reviews' }); },
  onWishList() {
    if (!this.data.wishCount) { wx.showToast({ title: '还没有想看的电影', icon: 'none' }); return; }
    wx.pageScrollTo({ selector: '.wish-card', duration: 200 });
  },
  onMovie(e) { wx.navigateTo({ url: '/pages/movie/movie?id=' + e.currentTarget.dataset.id }); },
  onMenu(e) {
    const key = e.currentTarget.dataset.key;
    if (key === 'orders') this.onTickets();
    else if (key === 'coupons') wx.navigateTo({ url: '/pages/coupons/coupons' });
    else if (key === 'card') wx.navigateTo({ url: '/pages/card/card' });
    else if (key === 'cinema') wx.navigateTo({ url: '/pages/cinema/cinema?id=' + store.favCinema() });
    else if (key === 'service') host.call(SERVICE_PHONE);
    else if (key === 'pref') {
      wx.showActionSheet({
        itemList: PREFS,
        success: r => {
          store.setPref(PREFS[r.tapIndex]);
          wx.showToast({ title: '已保存观影偏好', icon: 'none' });
          this.onShow();
        }
      });
    }
  }
});
