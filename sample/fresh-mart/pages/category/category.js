const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');

Page({
  data: {
    categories: data.categories,
    cate: 'fruit',
    cur: data.categories[0],
    sorts: ['综合', '销量', '价格从低到高', '价格从高到低', '有优惠'],
    sortIdx: 0,
    list: [],
    counts: {},
    bodyHeight: 500,
    mainTop: 0
  },

  onLoad() {
    const info = wx.getWindowInfo();
    // 兼容：本引擎 tab 页的 windowHeight 不扣 tabBar（微信会扣），见 README 的引擎问题说明
    const tabGap = info.screenHeight - info.windowHeight > 90 ? 0 : 50;
    this.setData({ bodyHeight: Math.floor(info.windowHeight - tabGap - 104 * info.windowWidth / 750) });
  },

  onShow() {
    // 首页金刚区点进来会带上分类
    const pick = wx.getStorageSync('fm_cate');
    if (pick) {
      wx.removeStorageSync('fm_cate');
      this.setData({ cate: pick, sortIdx: 0 });
    }
    this.load();
  },

  load() {
    const cur = data.categories.find(c => c.id === this.data.cate) || data.categories[0];
    let list = data.goods.filter(x => x.cat === cur.id);
    const idx = this.data.sortIdx;
    if (idx === 1) list = list.slice().sort((a, b) => b.sales - a.sales);
    if (idx === 2) list = list.slice().sort((a, b) => a.price - b.price);
    if (idx === 3) list = list.slice().sort((a, b) => b.price - a.price);
    if (idx === 4) list = list.filter(x => data.flashOf(x.id) || x.origin - x.price >= 10);
    const counts = {};
    cart.get().forEach(x => { counts[x.id] = (counts[x.id] || 0) + x.qty; });
    this.setData({ cur, list, counts, mainTop: this.data.mainTop ? 0 : 0.1 });
  },

  onCate(e) {
    this.setData({ cate: e.currentTarget.dataset.id, sortIdx: 0 });
    this.load();
  },

  onSort(e) {
    this.setData({ sortIdx: Number(e.currentTarget.dataset.idx) });
    this.load();
  },

  onSearch() {
    wx.navigateTo({ url: '/pages/search/search' });
  },

  onDetail(e) {
    wx.navigateTo({ url: `/pages/detail/detail?id=${e.currentTarget.dataset.id}` });
  },

  onAdd(e) {
    cart.add(e.currentTarget.dataset.id, 1);
    this.load();
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  }
});
