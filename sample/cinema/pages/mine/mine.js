const app = getApp();
Page({
  data: {
    wishes: [], wishCount: 0, seen: 23, ticketCount: 0, vip: false,
    menus: [
      { name: '优惠券', g: '券', extra: '3 张可用', bg: 'linear-gradient(135deg, #FF9A8B, #E8243F)' },
      { name: '卖品订单', g: '食', extra: '', bg: 'linear-gradient(135deg, #FFD36B, #F08A24)' },
      { name: '常去影院', g: '院', extra: '万达影城', bg: 'linear-gradient(135deg, #8EC5FF, #3D7BFF)' },
      { name: '观影偏好', g: '好', extra: 'IMAX · 中间排', bg: 'linear-gradient(135deg, #C7B2FF, #6A4BD8)' },
      { name: '联系客服', g: '服', extra: '', bg: 'linear-gradient(135deg, #8DE0B9, #1E9E6A)' }
    ]
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    const ids = wx.getStorageSync('cinema_wish') || [];
    this.setData({
      wishes: ids.map(id => app.getMovie(id)), wishCount: ids.length,
      ticketCount: app.getTickets().length + 1, vip: !!wx.getStorageSync('cinema_vip')
    });
  },
  onVip() {
    if (this.data.vip) return;
    wx.showModal({ title: '开通光影会员', content: '¥19/月，每周三购票立减 20 元，全年免服务费。', confirmText: '开通', success: (r) => {
      if (!r.confirm) return;
      wx.setStorageSync('cinema_vip', true); this.setData({ vip: true });
      wx.showToast({ title: '开通成功', icon: 'success' });
    } });
  },
  onTickets() { wx.switchTab({ url: '/pages/tickets/tickets' }); },
  onWishList() { if (!this.data.wishCount) wx.showToast({ title: '还没有想看的电影', icon: 'none' }); },
  onMovie(e) { wx.navigateTo({ url: '/pages/movie/movie?id=' + e.currentTarget.dataset.id }); },
  onMenu(e) { wx.showToast({ title: e.currentTarget.dataset.name, icon: 'none' }); }
});
