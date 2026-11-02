const D = require('../../utils/data.js');
const store = require('../../utils/store.js');

const SORTS = [
  { key: 'all', name: '综合' },
  { key: 'rating', name: '评分最高' },
  { key: 'years', name: '从业最久' },
  { key: 'fee', name: '设计费最低' }
];

Page({
  data: { styles: D.STYLES, style: '全部', sorts: SORTS, sort: 'all', list: [] },

  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 1 });
    this.load();
  },

  load() {
    const follows = store.get(store.K.favDesigners, []);
    let list = D.DESIGNERS.filter((d) => this.data.style === '全部' || d.styles.indexOf(this.data.style) >= 0).map((d) => {
      const works = D.CASES.filter((c) => c.designer === d.id);
      return Object.assign({}, d, { works: works.slice(0, 3), workCount: works.length, follow: follows.indexOf(d.id) >= 0, styleText: d.styles.join(' / ') });
    });
    const s = this.data.sort;
    if (s === 'rating') list.sort((a, b) => b.rating - a.rating || b.years - a.years);
    if (s === 'years') list.sort((a, b) => b.years - a.years);
    if (s === 'fee') list.sort((a, b) => a.fee - b.fee);
    this.setData({ list });
  },

  pickStyle(e) { this.setData({ style: e.currentTarget.dataset.s }, () => this.load()); },
  pickSort(e) { this.setData({ sort: e.currentTarget.dataset.k }, () => this.load()); },
  open(e) { wx.navigateTo({ url: '/pages/designer/designer?id=' + e.currentTarget.dataset.id }); },
  book(e) { wx.navigateTo({ url: '/pages/booking/booking?designer=' + e.currentTarget.dataset.id }); }
});
