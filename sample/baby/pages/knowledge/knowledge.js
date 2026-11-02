const store = require('../../utils/store.js');
const T = require('../../utils/time.js');
const KN = require('../../utils/knowledge.js');

Page({
  data: { stages: KN.STAGES, stage: '', mine: '', favOnly: false, list: [], favCount: 0, kw: '' },

  onLoad() {
    const baby = store.profile();
    const mine = KN.stageOf(T.age(baby.birthday).months);
    this.setData({ stage: mine, mine });
  },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 3 });
    // 「我的 → 我的收藏」跳过来时带的一次性标记（switchTab 不能带参数）
    if (wx.getStorageSync('kn_open_fav')) {
      wx.removeStorageSync('kn_open_fav');
      this.setData({ favOnly: true });
    }
    this.load();
  },

  load() {
    const favs = store.get(store.K.favs, []);
    const kw = this.data.kw.trim();
    let list = KN.ARTICLES.filter((a) => (this.data.favOnly ? favs.indexOf(a.id) >= 0 : a.stage === this.data.stage));
    if (kw) list = KN.ARTICLES.filter((a) => a.title.indexOf(kw) >= 0 || a.summary.indexOf(kw) >= 0 || a.tag === kw);
    list = list.map((a) => Object.assign({}, a, { fav: favs.indexOf(a.id) >= 0 }));
    this.setData({ list, favCount: favs.length });
  },

  pickStage(e) { this.setData({ stage: e.currentTarget.dataset.k, favOnly: false, kw: '' }, () => this.load()); },
  toggleFav() { this.setData({ favOnly: !this.data.favOnly, kw: '' }, () => this.load()); },
  onKw(e) { this.setData({ kw: e.detail.value }, () => this.load()); },
  clearKw() { this.setData({ kw: '' }, () => this.load()); },
  open(e) { wx.navigateTo({ url: '/pages/article/article?id=' + e.currentTarget.dataset.id }); }
});
