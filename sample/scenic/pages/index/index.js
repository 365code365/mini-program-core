const data = require('../../utils/data.js');
const tab = require('../../utils/tab.js');
const store = require('../../utils/store.js');
const CATS = ['全部', '山岳', '人文', '峡谷', '湖泊'];

Page({
  data: { banners: [], list: [], cats: CATS, cat: 0, kw: '', upcoming: null },
  onLoad() {
    this.setData({ banners: data.SCENIC.map((s) => ({ id: s.id, cover: s.cover, name: s.name, slogan: s.slogan })) });
    this.filter();
  },
  onShow() {
    tab.sync(this, 0);
    // 有待使用的门票：首页顶部给一个快捷入口
    const o = store.orders().filter((x) => x.status === 'paid').sort((a, b) => (a.date < b.date ? -1 : 1))[0];
    this.setData({ upcoming: o ? { no: o.no, name: o.scenicName, date: o.date, count: o.count } : null });
  },
  filter() {
    const kw = this.data.kw.trim(), cat = CATS[this.data.cat];
    const list = data.SCENIC.filter((s) => (cat === '全部' || s.tag === cat) && (!kw || s.name.indexOf(kw) >= 0 || s.city.indexOf(kw) >= 0))
      .map((s) => ({ id: s.id, name: s.name, city: s.city, level: s.level, tag: s.tag, score: s.score, comments: s.comments, cover: s.cover,
        from: Math.min(...s.tickets.map((t) => t.price)), intro: s.slogan }));
    this.setData({ list });
  },
  onInput(e) { this.setData({ kw: e.detail.value }); this.filter(); },
  onClear() { this.setData({ kw: '' }); this.filter(); },
  onCat(e) { this.setData({ cat: +e.currentTarget.dataset.i }); this.filter(); },
  goDetail(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  goTicket() { wx.navigateTo({ url: '/pages/ticket/ticket?no=' + this.data.upcoming.no }); }
});
