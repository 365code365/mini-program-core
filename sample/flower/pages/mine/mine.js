const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: { counts: { paid: 0, shipping: 0, done: 0 }, favs: [], remind: true, dates: [], member: {}, points: 0, couponCount: 0, addrCount: 0 },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 3 });
    const counts = { paid: 0, shipping: 0, done: 0 };
    store.getOrders().forEach(o => { if (counts[o.status] !== undefined) counts[o.status]++; });
    const favs = (wx.getStorageSync('fl_fav') || []).map(data.productOf).filter(Boolean);
    this.setData({
      counts, favs, remind: wx.getStorageSync('fl_remind') !== false, dates: store.datesView(),
      member: store.member(), points: store.points(), couponCount: store.validCoupons().length, addrCount: store.getAddrs().length
    });
  },
  // switchTab 带不了参数：要看的状态放在 storage 里，订单页 onShow 时读走
  goOrders(e) {
    const tab = e.currentTarget.dataset.tab;
    if (tab) wx.setStorageSync(store.K.orderTab, tab);
    wx.switchTab({ url: '/pages/orders/orders' });
  },
  goDetail(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  onRemind(e) {
    wx.setStorageSync('fl_remind', e.detail.value);
    this.setData({ remind: e.detail.value });
    wx.showToast({ title: e.detail.value ? '纪念日前 7 天会在首页提醒你订花' : '已关闭纪念日提醒', icon: 'none' });
  },
  addDate() { wx.navigateTo({ url: '/pages/anniv/anniv' }); },
  editDate(e) { wx.navigateTo({ url: '/pages/anniv/anniv?id=' + e.currentTarget.dataset.id }); },
  // 为这个纪念日订花：首页按对应用途筛好
  orderFor(e) {
    wx.setStorageSync(store.K.usePick, e.currentTarget.dataset.kind);
    wx.switchTab({ url: '/pages/index/index' });
  },
  menu(e) {
    const k = e.currentTarget.dataset.k;
    if (k === 'addr') wx.navigateTo({ url: '/pages/address/address' });
    else if (k === 'coupon') wx.navigateTo({ url: '/pages/coupons/coupons' });
    else if (k === 'vip') this.onVip();
    else this.onService();
  },
  onVip() {
    const m = this.data.member;
    wx.showModal({
      title: m.name + ' · ' + m.rateText,
      content: '累计消费 ¥' + m.spent + '。' + (m.gold ? '你已是金卡会员，全场花束 9 折。' : '银卡会员全场 9.5 折，再消费 ¥' + m.need + ' 升级金卡，享 9 折。') + '会员折扣可与优惠券同享，每消费 1 元得 1 积分。',
      showCancel: false, confirmText: '知道了'
    });
  },
  onService() {
    const items = data.faqs.map(f => f.q).concat(['拨打客服热线 ' + data.SERVICE_PHONE]);
    wx.showActionSheet({
      itemList: items,
      success: r => {
        if (r.tapIndex === data.faqs.length) { host.call(data.SERVICE_PHONE, '花语鲜花客服'); return; }
        const f = data.faqs[r.tapIndex];
        wx.showModal({ title: f.q, content: f.a, showCancel: false, confirmText: '知道了' });
      }
    });
  }
});
