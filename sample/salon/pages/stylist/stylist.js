const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

Page({
  data: { s: null, works: [], reviews: [], fav: false, avg: 0, tab: 'works', services: [], filterScore: 0 },
  onLoad(q) {
    const s = data.stylist(q.id) || data.stylists[0];
    this.sid = s.id;
    const works = s.works.map(id => data.work(id));
    const services = data.services.filter(x => s.cats.indexOf(x.cat) >= 0).slice(0, 6);
    this.setData({ s, works, services, tab: q.tab || 'works' });
    wx.setNavigationBarTitle({ title: s.name + ' · ' + s.title });
  },
  onShow() { this.loadReviews(); this.setData({ fav: store.favs().indexOf(this.sid) >= 0 }); },
  // 门店评价 + 我自己在「我的预约」里写的评价
  loadReviews() {
    const mine = store.appts().filter(a => a.stylistId === this.sid && a.review)
      .map(a => ({ id: 'u' + a.id, user: '我', score: a.review.score, date: a.review.date, service: a.serviceName, text: a.review.text, mine: true }));
    const all = mine.concat(data.reviews.filter(r => r.stylist === this.sid));
    const avg = all.length ? all.reduce((t, r) => t + r.score, 0) / all.length : 0;
    const f = this.data.filterScore;
    this.setData({ reviews: f ? all.filter(r => r.score >= f) : all, total: all.length, avg: Math.round(avg * 10) / 10 });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.t }); },
  onFilter(e) { this.setData({ filterScore: Number(e.currentTarget.dataset.v) }); this.loadReviews(); },
  preview(e) {
    const i = Number(e.currentTarget.dataset.i);
    wx.previewImage({ current: this.data.works[i].src, urls: this.data.works.map(w => w.src) });
  },
  toggleFav() {
    const on = store.toggleFav(this.sid);
    this.setData({ fav: on });
    wx.showToast({ title: on ? '已收藏，可在「发型师-我的收藏」找到' : '已取消收藏', icon: 'none' });
  },
  bookService(e) { wx.navigateTo({ url: `/pages/booking/booking?stylistId=${this.sid}&serviceId=${e.currentTarget.dataset.id}` }); },
  book() { wx.navigateTo({ url: '/pages/booking/booking?stylistId=' + this.sid }); },
  onShareAppMessage() { return { title: '推荐发型师 ' + this.data.s.name, path: '/pages/stylist/stylist?id=' + this.sid }; }
});
