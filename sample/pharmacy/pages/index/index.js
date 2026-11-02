// 首页：搜索入口、活动横幅、科室宫格（switchTab 不能带参数，科室放 globalData 给分类页读）、
// 今日用药提醒、常备药（加购抛物线小球落到 tabBar 的购物车上，角标弹一下）、健康资讯
const { DEPTS, BY_ID, ARTICLES, card } = require('../../utils/drugs.js');
const S = require('../../utils/store.js');
const F = require('../../utils/fly.js');

const STAPLE = ['d1', 'd3', 'd5', 'd18', 'd16', 'd24'];

Page({
  data: {
    depts: DEPTS, articles: ARTICLES, staples: [], doses: { total: 0, left: 0, next: null },
    banners: [
      { id: 'b1', img: '/images/banner/b1.jpg', t: '秋冬流感季', s: '退烧止咳常备药 满 99 减 10', go: 'cold' },
      { id: 'b2', img: '/images/banner/b2.jpg', t: '慢病复购', s: '处方药在线审方 · 药师 1 对 1', go: 'chronic' }
    ],
    ball: { on: false }, ballX: {}, ballY: {}
  },
  onLoad() {
    this.setData({ staples: STAPLE.map(id => card(BY_ID[id])) });
  },
  onShow() {
    F.syncTab(this, 0, S.cartCount());
    const list = S.todayDoses();
    const left = list.filter(d => !d.taken);
    this.setData({ doses: { total: list.length, left: left.length, next: left[0] || null } });
  },
  goSearch() { wx.navigateTo({ url: '/pages/search/search' }); },
  goDept(e) {
    getApp().globalData.dept = e.currentTarget.dataset.id;
    wx.switchTab({ url: '/pages/category/category' });
  },
  openDrug(e) { wx.navigateTo({ url: '/pages/drug/drug?id=' + e.currentTarget.dataset.id }); },
  openArticle(e) { wx.navigateTo({ url: '/pages/article/article?id=' + e.currentTarget.dataset.id }); },
  goRemind() { wx.navigateTo({ url: '/pages/remind/remind' }); },
  goRx() { wx.navigateTo({ url: '/pages/rx/rx' }); },
  addCart(e) {
    const id = e.currentTarget.dataset.id;
    const n = S.addCart(id, 0, 1);
    const win = wx.getWindowInfo();
    // 落点：tabBar 第 3 格（购物车）图标中心
    F.fly(this, e, { x: win.windowWidth * 5 / 8, y: win.windowHeight - 32 }, () => {
      const tb = this.getTabBar && this.getTabBar();
      if (tb && tb.bump) tb.bump(n); else F.syncTab(this, 0, n);
    });
    if (BY_ID[id].type === 'RX') wx.showToast({ title: '处方药结算前需审方', icon: 'none' });
  },
  onShareAppMessage() { return { title: '康源大药房 · 常备药 30 分钟送到家', path: '/pages/index/index' }; }
});
