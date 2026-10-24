const app = getApp();
const u = app.util;
const CITIES = ['杭州', '上海', '北京', '成都', '三亚', '厦门', '大理', '西安'];

Page({
  data: {
    kinds: ['国内酒店', '民宿', '钟点房'], kind: 0,
    cities: CITIES, cityIdx: 0,
    prices: ['不限', '¥300以下', '¥300-600', '¥600-1000', '¥1000以上'],
    stars: ['不限', '经济型', '舒适型', '高档型', '豪华型', '特色民宿'],
    perks: [
      { g: '保', t: '低价保障', s: '买贵退差', bg: 'linear-gradient(135deg, #6BA8FF, #1E6FFF)' },
      { g: '退', t: '免费取消', s: '入住前可退', bg: 'linear-gradient(135deg, #5FD99A, #1E9E6A)' },
      { g: '会', t: '会员 9 折', s: '积分抵现', bg: 'linear-gradient(135deg, #FFC35C, #F08A24)' },
      { g: '速', t: '闪住', s: '免押金', bg: 'linear-gradient(135deg, #B28CFF, #6A4BD8)' }
    ],
    recs: app.HOTELS.slice(0, 5), s: {}
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    this.apply(app.getSearch());
  },
  apply(s) {
    const today = u.fmt(new Date());
    const max = new Date(); max.setDate(max.getDate() + 180);
    const minOut = u.fmt(new Date(u.parse(s.checkin).getTime() + 86400000));
    this.setData({
      s, today, maxDate: u.fmt(max), minOut, cityIdx: Math.max(0, CITIES.indexOf(s.city)),
      inShort: u.short(s.checkin), inWeek: u.week(s.checkin), outShort: u.short(s.checkout), outWeek: u.week(s.checkout),
      nights: u.nights(s.checkin, s.checkout)
    });
    app.saveSearch(s);
  },
  patch(p) { this.apply(Object.assign({}, this.data.s, p)); },
  onKind(e) {
    const idx = e.currentTarget.dataset.idx;
    this.setData({ kind: idx });
    if (idx === 1) this.patch({ star: 5 });
    if (idx === 2) wx.showToast({ title: '钟点房 4 小时起订', icon: 'none' });
  },
  onCity(e) { this.patch({ city: CITIES[e.detail.value] }); },
  onCheckin(e) {
    const ci = e.detail.value;
    let co = this.data.s.checkout;
    if (co <= ci) co = u.fmt(new Date(u.parse(ci).getTime() + 86400000));
    this.patch({ checkin: ci, checkout: co });
  },
  onCheckout(e) { this.patch({ checkout: e.detail.value }); },
  onKeyword(e) { this.data.s.keyword = e.detail.value; app.saveSearch(this.data.s); },
  onPrice(e) { this.patch({ price: e.currentTarget.dataset.idx }); },
  onStar(e) { this.patch({ star: e.currentTarget.dataset.idx }); },
  onSearch() {
    if (this.data.s.city !== '杭州') {
      wx.showModal({ title: '提示', content: '演示版仅收录了杭州的酒店，将为你展示杭州的结果。', showCancel: false, success: () => wx.navigateTo({ url: '/pages/list/list' }) });
      return;
    }
    wx.navigateTo({ url: '/pages/list/list' });
  },
  onHotel(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); }
});
