const store = require('../../utils/store.js');
const social = require('../../utils/social.js');
Page({
  data: { c: {}, joined: false, sort: 'new', list: [], empty: false },
  onLoad(q) {
    const c = store.circle(q.id);
    this.cid = c.id;
    wx.setNavigationBarTitle({ title: c.name + '圈' });
    this.setData({ c: { name: c.name, desc: c.desc, color: c.color, initial: c.name.slice(0, 1), from: store.tint(c.color, 0.75) } });
  },
  onShow() { this.load(); },
  load() {
    const j = store.joined(); const likes = store.likes(); const favs = store.favs();
    let src = store.posts().filter((p) => p.circle === this.cid);
    if (this.data.sort === 'hot') src = src.slice().sort((a, b) => (b.likes + store.card(b, likes, favs).comments * 3) - (a.likes + store.card(a, likes, favs).comments * 3));
    const list = src.map((p) => store.card(p, likes, favs));
    const base = store.CIRCLES.find((x) => x.id === this.cid).members;
    this.setData({ list, empty: !list.length, joined: j.indexOf(this.cid) >= 0, members: store.fmtNum(base + (j.indexOf(this.cid) >= 0 ? 1 : 0)), count: list.length });
  },
  setSort(e) { this.setData({ sort: e.currentTarget.dataset.s }); this.load(); },
  toggleJoin() {
    const j = store.joined();
    const on = j.indexOf(this.cid) >= 0;
    wx.setStorageSync('forum_joined', on ? j.filter((x) => x !== this.cid) : j.concat([this.cid]));
    this.load();
    wx.showToast({ title: on ? '已退出' : '已加入', icon: 'none' });
  },
  open(e) { wx.navigateTo({ url: '/pages/post/post?id=' + e.detail.id }); },
  like(e) { social.toggle(this, 'list', 'like', e.detail.id); },
  fav(e) { social.toggle(this, 'list', 'fav', e.detail.id); },
  noop() {}
});
