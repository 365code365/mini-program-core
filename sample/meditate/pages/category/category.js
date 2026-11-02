const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const theme = require('../../utils/theme.js');

Page({
  data: { theme: 'light', cats: data.CATS, cur: 'all', list: [], loading: true, skel: [1, 2, 3] },

  onShow() {
    this.applyTheme();
    // 「我的 → 我的收藏」切过来时带上分类（switchTab 不能带 query）
    const app = getApp();
    const jump = app.globalData.catJump;
    if (jump) { app.globalData.catJump = ''; this.setData({ cur: jump }); }
    this.load(this.data.list.length === 0);
  },

  applyTheme() { theme.apply(this, 1); },

  onCat(e) {
    const key = e.currentTarget.dataset.key;
    if (key === this.data.cur) return;
    this.setData({ cur: key });
    this.load(true);
  },

  showAll() { this.setData({ cur: 'all' }); this.load(true); },

  // 列表按分类过滤；切分类时先给 300ms 骨架屏（真实场景是拉接口）
  load(withSkeleton) {
    const build = () => {
      const cur = this.data.cur;
      const favs = store.favs();
      const done = store.doneMap();
      const list = data.COURSES
        .filter(c => cur === 'all' || (cur === 'fav' ? favs.indexOf(c.id) >= 0 : c.cat === cur))
        .map(c => {
          const n = (done[c.id] || []).length;
          return Object.assign({}, c, {
            doneN: n,
            pct: Math.round(n / c.chapters.length * 100),
            learnersText: c.learners >= 10000 ? (c.learners / 10000).toFixed(1) + '万' : String(c.learners)
          });
        });
      this.setData({ list, loading: false });
    };
    if (!withSkeleton) return build();
    this.setData({ loading: true, skel: [1, 2, 3] });
    clearTimeout(this._t);
    this._t = setTimeout(build, 300);
  },

  open(e) { wx.navigateTo({ url: '/pages/course/course?id=' + e.currentTarget.dataset.id }); },

  onUnload() { clearTimeout(this._t); }
});
