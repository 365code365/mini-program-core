const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

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
    // 首页金刚区、优惠券「去使用」点进来会带上分类
    const pick = wx.getStorageSync('fm_cate');
    if (pick) {
      wx.removeStorageSync('fm_cate');
      this.setData({ cate: pick, sortIdx: 0 });
    }
    this.load();
  },

  // 自定义 tabBar：微信里每个 tab 页一个实例，选中态与购物车角标要各自设
  syncBar() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1, cartCount: store.cartCount() });
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
    store.getCart().forEach(x => { counts[x.id] = (counts[x.id] || 0) + x.qty; });
    this.setData({ cur, list, counts, mainTop: this.data.mainTop ? 0 : 0.1 });
    this.syncBar();
  },

  onCate(e) {
    this.setData({ cate: e.currentTarget.dataset.id, sortIdx: 0 });
    this.load();
  },

  onSort(e) {
    this.setData({ sortIdx: Number(e.currentTarget.dataset.idx) });
    this.load();
  },

  // 「有优惠」筛空了：回到综合排序
  onResetSort() {
    this.setData({ sortIdx: 0 });
    this.load();
  },

  onSearch() {
    wx.navigateTo({ url: '/pages/search/search' });
  },

  onDetail(e) {
    wx.navigateTo({ url: `/pages/detail/detail?id=${e.currentTarget.dataset.id}` });
  },

  onAdd(e) {
    const r = store.addToCart(e.currentTarget.dataset.id, 1);
    this.load();
    if (r.ok && !r.msg) wx.showToast({ title: '已加入购物车', icon: 'success' });
    else wx.showToast({ title: r.msg, icon: 'none' });
  }
});
