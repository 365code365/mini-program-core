const { goods, stories, live, farmers } = require('../../utils/data.js');
const store = require('../../utils/store.js');
const ticker = require('../../utils/ticker.js');

function soldText(n) { return n >= 10000 ? (n / 10000).toFixed(1) + '万' : String(n); }
// 当月在产季里的算时令（产季写的是「11月—次年2月」这种区间）
function inSeason(g, m) {
  const r = /(\d+)月(?:—(?:次年)?(\d+)月)?/.exec(g.season);
  if (!r) return true;
  const a = +r[1], b = r[2] ? +r[2] : a;
  return a <= b ? m >= a && m <= b : m >= a || m <= b;
}

Page({
  data: { stories, live: {}, season: [], groups: [], list: [], sort: 'hot', monthText: '', groupHint: '' },

  onLoad() {
    const f = farmers[live.farmer];
    const m = new Date().getMonth() + 1;
    let season = goods.filter((g) => inSeason(g, m));
    if (season.length < 4) season = goods.slice(0, 6);
    this.setData({
      monthText: m + '月',
      live: { cover: live.cover, title: live.title, viewersText: soldText(live.viewers), farmerName: f.name, farmerTitle: f.title },
      season: season.map((g) => ({ id: g.id, cover: g.cover, price: g.price, short: g.name.split(' ')[0] }))
    });
    this.applySort('hot');
    this.buildGroups();
  },

  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 0, badge: store.cartCount() });
    // tab 页不会 onUnload：显示时订阅、隐藏时退订，几个倒计时共用 ticker 里的一个定时器
    ticker.attach(this, (now) => this.tick(now));
    this.tick(Date.now());
  },
  onHide() { ticker.detach(this); },
  onUnload() { ticker.detach(this); },

  buildGroups() {
    // 每个拼团商品取最早结束的那个公开团做倒计时
    const picks = ['g01', 'g07', 'g03'].map((id) => {
      const g = goods.find((x) => x.id === id);
      const og = store.openGroups(id)[0];
      return { id: 'hg-' + id, goodsId: id, name: g.name, cover: g.cover, size: g.groupSize, price: g.price, groupPrice: g.groupPrice, soldText: soldText(g.sold), endAt: og ? og.endAt : Date.now() + 3600e3, cd: null };
    });
    this._groups = picks;
    this.setData({ groups: picks, groupHint: '每人限参 1 团' });
  },

  tick(now) {
    if (!this._groups) return;
    const patch = {};
    this._groups.forEach((g, i) => { patch[`groups[${i}].cd`] = ticker.left(g.endAt, now); });
    this.setData(patch);
  },

  setSort(e) { this.applySort(e.currentTarget.dataset.s); },
  applySort(sort) {
    const list = goods.slice().sort((a, b) => (sort === 'low' ? a.price - b.price : b.sold - a.sold))
      .map((g) => ({ id: g.id, name: g.name, origin: g.origin, cover: g.cover, price: g.price, soldText: soldText(g.sold) }));
    this.setData({ sort, list });
  },

  onStoryChange(e) { this._story = e.detail.current; },
  openStory(e) { wx.navigateTo({ url: '/pages/story/story?id=' + e.currentTarget.dataset.id }); },
  openGoods(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  goLive() { wx.switchTab({ url: '/pages/live/live' }); },
  goGroupHall() { wx.navigateTo({ url: '/pages/group/group' }); },

  quickAdd(e) {
    const g = goods.find((x) => x.id === e.currentTarget.dataset.id);
    const key = Object.keys(g.sku.map).find((k) => g.sku.map[k].stock > 0);
    store.addCart(g.id, key, 1);
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ badge: store.cartCount() });
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },

  onShareAppMessage() { return { title: '田园直供｜产地直发的时令好物', path: '/pages/index/index' }; }
});
