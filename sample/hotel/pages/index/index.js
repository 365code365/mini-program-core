const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const CITIES = data.CITIES.map(c => c.name);
const PERKS = [
  { g: '保', t: '低价保障', s: '买贵退差', bg: 'linear-gradient(135deg, #6BA8FF, #1E6FFF)', info: '预订后 24 小时内，在其他平台发现同酒店、同房型、同日期的更低价，联系客服核实后退还差价。' },
  { g: '退', t: '免费取消', s: '入住前可退', bg: 'linear-gradient(135deg, #5FD99A, #1E9E6A)', info: '入住前一天 18:00 前可免费取消，当天的订单下单后 30 分钟内可免费取消，款项原路退回。' },
  { g: '会', t: '金卡 95 折', s: '积分抵现', bg: 'linear-gradient(135deg, #FFC35C, #F08A24)', info: '金卡会员订房享 9.5 折；100 积分抵 1 元，单笔最多抵 50 元。' },
  { g: '速', t: '闪住', s: '免押金', bg: 'linear-gradient(135deg, #B28CFF, #6A4BD8)', info: '金卡会员入住免押金，离店无需查房，账单自动结算。' }
];
Page({
  data: {
    kinds: ['国内酒店', '民宿', '钟点房'],
    cities: CITIES, cityIdx: 0,
    prices: ['不限', '¥300以下', '¥300-600', '¥600-1000', '¥1000以上'],
    stars: ['不限', '经济型', '舒适型', '高档型', '豪华型', '特色民宿'],
    perks: PERKS, recs: [], s: {}, panel: false, pp: { rooms: 1, adults: 2, children: 0 }
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    this.apply(store.getSearch());
  },
  apply(s) {
    const t = store.today();
    this.setData({
      s, today: t, maxDate: store.addDays(t, 180), minOut: store.addDays(s.checkin, 1), cityIdx: Math.max(0, CITIES.indexOf(s.city)),
      inShort: store.short(s.checkin), inWeek: store.week(s.checkin), outShort: store.short(s.checkout), outWeek: store.week(s.checkout),
      nights: store.nights(s.checkin, s.checkout), people: store.peopleText(s),
      recs: data.HOTELS.filter(h => h.city === s.city).sort((a, b) => b.score - a.score || b.reviews - a.reviews).slice(0, 5)
    });
    store.saveSearch(s);
  },
  patch(p) { this.apply(Object.assign({}, this.data.s, p)); },
  // 民宿：星级筛到「特色民宿」；切回酒店时把这条筛选去掉
  onKind(e) {
    const kind = Number(e.currentTarget.dataset.idx);
    const s = this.data.s;
    this.patch({ kind, star: kind === 1 ? 5 : (s.star === 5 ? 0 : s.star) });
  },
  onCity(e) { this.patch({ city: CITIES[Number(e.detail.value)] }); },
  // 我的位置：拿到定位就挑最近的城市；宿主不给定位时按常住城市（杭州）处理
  onLocate() {
    host.locate().then(pos => {
      if (!pos) {
        this.patch({ city: '杭州' });
        wx.showToast({ title: '无法获取定位，已切换到常住城市杭州', icon: 'none' });
        return;
      }
      let best = data.CITIES[0];
      let bd = 1e9;
      data.CITIES.forEach(c => { const d = Math.pow(c.lat - pos.lat, 2) + Math.pow(c.lng - pos.lng, 2); if (d < bd) { bd = d; best = c; } });
      this.patch({ city: best.name });
      wx.showToast({ title: '已定位到 ' + best.name, icon: 'none' });
    });
  },
  onCheckin(e) {
    const ci = e.detail.value;
    let co = this.data.s.checkout;
    if (co <= ci) co = store.addDays(ci, 1);
    this.patch({ checkin: ci, checkout: co });
  },
  onCheckout(e) { this.patch({ checkout: e.detail.value }); },
  onKeyword(e) { this.data.s.keyword = e.detail.value.trim(); store.saveSearch(this.data.s); },
  onPrice(e) { this.patch({ price: Number(e.currentTarget.dataset.idx) }); },
  onStar(e) { this.patch({ star: Number(e.currentTarget.dataset.idx) }); },
  onPeople() {
    const s = this.data.s;
    this.setData({ panel: true, pp: { rooms: s.rooms, adults: s.adults, children: s.children } });
  },
  closePanel() { this.setData({ panel: false }); },
  noop() {},
  ppStep(e) {
    const k = e.currentTarget.dataset.k;
    const d = Number(e.currentTarget.dataset.d);
    const pp = Object.assign({}, this.data.pp);
    const lim = { rooms: [1, 5], adults: [1, 10], children: [0, 5] }[k];
    const v = pp[k] + d;
    if (v < lim[0] || v > lim[1]) return;
    pp[k] = v;
    // 每间至少住 1 位成人
    if (pp.rooms > pp.adults) { if (k === 'rooms') pp.adults = pp.rooms; else return; }
    this.setData({ pp });
  },
  onPeopleOk() { this.patch(this.data.pp); this.closePanel(); },
  onSearch() { wx.navigateTo({ url: '/pages/list/list' }); },
  onPerk(e) {
    const p = PERKS[Number(e.currentTarget.dataset.i)];
    wx.showModal({ title: p.t, content: p.info, showCancel: false, confirmText: '知道了' });
  },
  onHotel(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); }
});
