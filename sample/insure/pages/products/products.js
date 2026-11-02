const { CATS, PRODUCTS } = require('../../utils/products.js');
const premium = require('../../utils/premium.js');
const util = require('../../utils/util.js');

const AGES = [
  { name: '不限年龄', range: null },
  { name: '少儿 0~17 岁', range: [0, 17] },
  { name: '成人 18~50 岁', range: [18, 50] },
  { name: '中老年 51 岁以上', range: [51, 80] }
];
const SORTS = [{ key: 'default', name: '综合' }, { key: 'price', name: '价格最低' }, { key: 'sales', name: '销量最高' }];

Page({
  data: {
    cats: [{ key: '', name: '全部' }].concat(CATS.map(c => ({ key: c.key, name: c.name }))),
    cat: '', sorts: SORTS, sort: 'default',
    ageNames: AGES.map(a => a.name), ageIdx: 0,
    list: [], loading: true
  },
  onLoad() { this.refresh(); },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 1 });
    const g = getApp().globalData;
    if (g.productCat !== null && g.productCat !== undefined && g.productCat !== this.data.cat) {
      this.setData({ cat: g.productCat });
      this.refresh();
    }
    g.productCat = null;
  },
  refresh() {
    this.setData({ loading: true });
    clearTimeout(this._t);
    // 模拟接口耗时：筛选条件变了先出加载态
    this._t = setTimeout(() => {
      const { cat, sort, ageIdx } = this.data;
      const range = AGES[ageIdx].range;
      let list = PRODUCTS.filter(p => (!cat || p.cat === cat) && (!range || (p.ages[0] <= range[1] && p.ages[1] >= range[0])));
      list = list.map(p => ({
        id: p.id, name: p.name, cover: p.cover, tagline: p.tagline, tags: p.tags, catName: p.catName,
        ages: p.ages[0] + '~' + p.ages[1] + '岁', insurer: p.insurer.replace('股份有限公司', ''),
        priceNum: premium.priceFrom(p), price: util.money(premium.priceFrom(p)), unit: p.model === 'travel' ? '/次' : '/年',
        sales: p.sales, salesText: p.sales >= 10000 ? (p.sales / 10000).toFixed(1) + '万' : String(p.sales)
      }));
      if (sort === 'price') list.sort((a, b) => a.priceNum - b.priceNum);
      if (sort === 'sales') list.sort((a, b) => b.sales - a.sales);
      this.setData({ list, loading: false });
    }, 350);
  },
  onCat(e) {
    const key = e.currentTarget.dataset.key;
    if (key === this.data.cat) return;
    this.setData({ cat: key });
    this.refresh();
  },
  onSort(e) {
    this.setData({ sort: e.currentTarget.dataset.key });
    this.refresh();
  },
  onAge(e) {
    this.setData({ ageIdx: Number(e.detail.value) });
    this.refresh();
  },
  resetFilter() {
    this.setData({ cat: '', sort: 'default', ageIdx: 0 });
    this.refresh();
  },
  openDetail(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  openCalc(e) { wx.navigateTo({ url: '/pages/calc/calc?id=' + e.currentTarget.dataset.id }); }
});
