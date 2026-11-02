const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

Page({
  data: { svc: null, steps: [], notices: data.notices, stylists: [], memberPrice: 0, levelName: '', related: [] },
  onLoad(q) {
    const svc = data.service(q.id) || data.services[0];
    const lv = store.levelOf(store.member().growth).cur;
    const stylists = data.stylists.filter(s => s.cats.indexOf(svc.cat) >= 0);
    const related = data.services.filter(s => s.cat === svc.cat && s.id !== svc.id).slice(0, 3);
    this.setData({
      svc: Object.assign({}, svc, { cover: data.catCover[svc.cat], catName: data.catName(svc.cat) }),
      steps: data.steps[svc.cat] || [],
      stylists, related,
      memberPrice: Math.round(svc.price * lv.discount),
      levelName: lv.name
    });
    wx.setNavigationBarTitle({ title: svc.name });
  },
  goStylist(e) { wx.navigateTo({ url: '/pages/stylist/stylist?id=' + e.currentTarget.dataset.id }); },
  bookWith(e) {
    wx.navigateTo({ url: `/pages/booking/booking?serviceId=${this.data.svc.id}&stylistId=${e.currentTarget.dataset.id}` });
  },
  book() { wx.navigateTo({ url: '/pages/booking/booking?serviceId=' + this.data.svc.id }); },
  goRelated(e) { wx.redirectTo({ url: '/pages/service/service?id=' + e.currentTarget.dataset.id }); },
  goMember() { wx.navigateTo({ url: '/pages/member/member' }); },
  onShareAppMessage() { return { title: this.data.svc.name + ' ¥' + this.data.svc.price, path: '/pages/service/service?id=' + this.data.svc.id }; }
});
