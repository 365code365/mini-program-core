const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: { favs: [], favCount: 0, views: 0, points: 0, coupons: 0, signed: false, member: {}, menus: [] },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    const ids = store.favs();
    const rated = store.getOrders().filter(o => o.rating).length;
    this.setData({
      favs: ids.slice(0, 3).map(data.hotelOf), favCount: ids.length, views: store.views().length,
      points: store.getPoints(), coupons: store.validCoupons().length, signed: store.signedToday(), member: store.member(),
      menus: [
        { k: 'guests', n: '常用入住人', x: store.getGuests().length + ' 位' },
        { k: 'invoices', n: '发票抬头管理', x: store.getInvoices().length + ' 个' },
        { k: 'reviews', n: '我的点评', x: rated ? rated + ' 条' : '' },
        { k: 'service', n: '联系客服', x: '7×24 小时' },
        { k: 'about', n: '关于途栖', x: '' }
      ]
    });
  },
  onSign() {
    if (!store.sign()) { wx.showToast({ title: '今天已经签过了', icon: 'none' }); return; }
    this.setData({ signed: true, points: store.getPoints() });
    wx.showToast({ title: '签到成功 +10 积分', icon: 'none' });
  },
  onRights() {
    const m = this.data.member;
    wx.showModal({
      title: m.name + '权益',
      content: '订房 9.5 折、入住免押金、延迟退房至 13:00、免费早餐升级、积分抵现、专属客服。' + (m.need ? '再住 ' + m.need + ' 晚升级铂金卡，延迟退房至 14:00。' : ''),
      showCancel: false, confirmText: '知道了'
    });
  },
  onFavs() { wx.navigateTo({ url: '/pages/collect/collect?tab=fav' }); },
  onViews() { wx.navigateTo({ url: '/pages/collect/collect?tab=view' }); },
  onPoints() { wx.navigateTo({ url: '/pages/points/points' }); },
  onCoupons() { wx.navigateTo({ url: '/pages/coupons/coupons' }); },
  onHotel(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  onMenu(e) {
    const k = e.currentTarget.dataset.k;
    if (k === 'guests') wx.navigateTo({ url: '/pages/guests/guests' });
    else if (k === 'invoices') wx.navigateTo({ url: '/pages/invoices/invoices' });
    else if (k === 'reviews') wx.navigateTo({ url: '/pages/reviews/reviews?mine=1' });
    else if (k === 'service') this.onService();
    else wx.showModal({ title: '关于途栖', content: '途栖酒店民宿 v3.2.0\n精选全国酒店与特色民宿，低价保障、到店无忧。\n客服热线 ' + data.SERVICE_PHONE, showCancel: false, confirmText: '知道了' });
  },
  onService() {
    const items = data.faqs.map(f => f.q).concat(['拨打客服热线 ' + data.SERVICE_PHONE]);
    wx.showActionSheet({
      itemList: items,
      success: r => {
        if (r.tapIndex === data.faqs.length) { host.call(data.SERVICE_PHONE, '途栖客服'); return; }
        const f = data.faqs[r.tapIndex];
        wx.showModal({ title: f.q, content: f.a, showCancel: false, confirmText: '知道了' });
      }
    });
  }
});
