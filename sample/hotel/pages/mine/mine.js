const app = getApp();
Page({
  data: { favs: [], views: 0, points: 1860, signed: false, menus: ['常用入住人', '发票抬头管理', '我的点评', '联系客服', '关于途栖'] },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    const ids = wx.getStorageSync('hotel_favs') || [];
    const today = app.util.fmt(new Date());
    const signed = wx.getStorageSync('hotel_signed') === today;
    this.setData({
      favs: ids.map(id => app.getHotel(id)),
      views: (wx.getStorageSync('hotel_views') || []).length,
      signed, points: 1860 + (signed ? 10 : 0)
    });
  },
  onSign() {
    if (this.data.signed) { wx.showToast({ title: '今天已经签过了', icon: 'none' }); return; }
    wx.setStorageSync('hotel_signed', app.util.fmt(new Date()));
    this.setData({ signed: true, points: this.data.points + 10 });
    wx.showToast({ title: '签到成功 +10', icon: 'success' });
  },
  onFavTab() { if (!this.data.favs.length) wx.showToast({ title: '暂无收藏', icon: 'none' }); },
  onHotel(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  onMenu(e) { wx.showToast({ title: e.currentTarget.dataset.name, icon: 'none' }); }
});
