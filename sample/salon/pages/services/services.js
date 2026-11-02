const data = require('../../utils/data.js');

Page({
  data: {
    cats: data.categories,
    cat: 'all',
    catView: 'cat-all',
    list: [],
    loading: true,
    keyword: ''
  },
  onLoad() { this.load('all'); },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 1 });
    // 从首页宫格 / 别的页面带过来的分类（switchTab 不能带 query，走 globalData）
    const app = getApp();
    if (app.globalData.serviceCat) {
      const c = app.globalData.serviceCat;
      app.globalData.serviceCat = '';
      this.load(c);
    }
  },
  load(cat) {
    this.setData({ cat, catView: 'cat-' + cat, loading: true });
    // 模拟接口耗时：切分类时先显示加载态
    clearTimeout(this._t);
    this._t = setTimeout(() => this.filter(), 260);
  },
  filter() {
    const kw = this.data.keyword.trim();
    const list = data.services
      .filter(s => this.data.cat === 'all' || s.cat === this.data.cat)
      .filter(s => !kw || s.name.indexOf(kw) >= 0 || s.desc.indexOf(kw) >= 0)
      .map(s => Object.assign({}, s, { cover: data.catCover[s.cat], catName: data.catName(s.cat),
        memberPrice: Math.round(s.price * 0.9) }));
    this.setData({ list, loading: false });
  },
  onCat(e) {
    const c = e.currentTarget.dataset.id;
    if (c !== this.data.cat) this.load(c);
  },
  onInput(e) { this.setData({ keyword: e.detail.value }); },
  onSearch() { this.setData({ loading: true }); setTimeout(() => this.filter(), 200); },
  clearKw() { this.setData({ keyword: '' }); this.filter(); },
  goService(e) { wx.navigateTo({ url: '/pages/service/service?id=' + e.currentTarget.dataset.id }); },
  book(e) { wx.navigateTo({ url: '/pages/booking/booking?serviceId=' + e.currentTarget.dataset.id }); },
  onPullDownRefresh() { this.filter(); wx.stopPullDownRefresh(); }
});
