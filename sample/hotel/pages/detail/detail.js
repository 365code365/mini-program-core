const app = getApp();
const u = app.util;
Page({
  data: { h: null, cur: 0, fav: false, reviews: app.REVIEWS },
  onLoad(q) {
    const h = app.getHotel(q.id);
    const favs = wx.getStorageSync('hotel_favs') || [];
    this.setData({ h, fav: favs.indexOf(h.id) >= 0 });
    wx.setNavigationBarTitle({ title: h.name.length > 12 ? h.name.slice(0, 12) + '…' : h.name });
    const views = (wx.getStorageSync('hotel_views') || []).filter(id => id !== h.id);
    views.unshift(h.id);
    wx.setStorageSync('hotel_views', views.slice(0, 10));
  },
  onShow() {
    const s = app.getSearch();
    this.setData({ inShort: u.short(s.checkin), inWeek: u.week(s.checkin), outShort: u.short(s.checkout), outWeek: u.week(s.checkout), nights: u.nights(s.checkin, s.checkout) });
  },
  onSwiper(e) { this.setData({ cur: e.detail.current }); },
  onFav() {
    let favs = wx.getStorageSync('hotel_favs') || [];
    const id = this.data.h.id;
    const fav = favs.indexOf(id) < 0;
    favs = fav ? [id].concat(favs) : favs.filter(x => x !== id);
    wx.setStorageSync('hotel_favs', favs);
    this.setData({ fav });
    wx.showToast({ title: fav ? '已收藏' : '已取消收藏', icon: fav ? 'success' : 'none' });
  },
  onCall() {
    wx.showActionSheet({ itemList: ['拨打酒店电话 0571-8799 6688', '在线咨询客服'] });
  },
  onMap() { wx.showToast({ title: this.data.h.addr, icon: 'none' }); },
  onDates() { wx.showToast({ title: '请在首页修改入住日期', icon: 'none' }); },
  onBook(e) {
    wx.navigateTo({ url: '/pages/booking/booking?hid=' + this.data.h.id + '&rid=' + e.currentTarget.dataset.rid });
  }
});
