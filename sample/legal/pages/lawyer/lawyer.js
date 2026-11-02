const D = require('../../utils/data.js');
const S = require('../../utils/store.js');

Page({
  data: { l: null, fav: false, services: [], reviews: [], stars: [1, 2, 3, 4, 5] },
  onLoad(q) {
    const raw = D.lawyerById(q.id) || D.lawyers[0];
    const l = D.decorate(raw);
    wx.setNavigationBarTitle({ title: l.name + ' 律师' });
    const services = D.services.map(s => Object.assign({}, s, { price: raw.price[s.key] }));
    this.setData({ l, services, fav: S.favs().indexOf(l.id) >= 0, fieldNames: raw.fields.map(D.catName) });
  },
  onShow() {
    if (!this.data.l) return;
    // 自己评价过的咨询排在最前（评价页写进 storage，回来就能看到）
    const mine = S.consults().filter(c => c.lawyerId === this.data.l.id && c.rating)
      .map(c => ({ user: '我', star: c.rating.star, text: c.rating.text || c.rating.tags.join('、'), date: c.rating.date }));
    this.setData({ reviews: mine.concat(this.data.l.reviews) });
  },
  toggleFav() {
    const on = S.toggleFav(this.data.l.id);
    this.setData({ fav: on });
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' });
  },
  order(e) {
    const s = (e.currentTarget.dataset.s) || 'text';
    wx.navigateTo({ url: `/pages/order/order?lawyer=${this.data.l.id}&service=${s}` });
  }
});
