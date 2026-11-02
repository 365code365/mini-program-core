const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

const FILTERS = [
  { id: 'all', name: '全部' },
  { id: 'hair', name: '美发' },
  { id: 'nail', name: '美甲美睫' },
  { id: 'fav', name: '我的收藏' }
];

Page({
  data: { filters: FILTERS, filter: 'all', list: [], sort: 'rating' },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 2 });
    const app = getApp();
    if (app.globalData.stylistFilter) {
      this.setData({ filter: app.globalData.stylistFilter });
      app.globalData.stylistFilter = '';
    }
    this.refresh();
  },
  refresh() {
    const favs = store.favs();
    const f = this.data.filter;
    let list = data.stylists.filter(s => {
      if (f === 'hair') return s.cats.indexOf('cut') >= 0 || s.cats.indexOf('dye') >= 0;
      if (f === 'nail') return s.cats.indexOf('nail') >= 0 || s.cats.indexOf('lash') >= 0;
      if (f === 'fav') return favs.indexOf(s.id) >= 0;
      return true;
    }).map(s => Object.assign({}, s, {
      fav: favs.indexOf(s.id) >= 0,
      catText: s.cats.map(c => data.catName(c)).join(' / '),
      cover: data.work(s.works[0]).src
    }));
    const key = this.data.sort;
    list.sort((a, b) => b[key] - a[key]);
    this.setData({ list });
  },
  onFilter(e) { this.setData({ filter: e.currentTarget.dataset.id }); this.refresh(); },
  onSort(e) { this.setData({ sort: e.currentTarget.dataset.k }); this.refresh(); },
  toggleFav(e) {
    const on = store.toggleFav(e.currentTarget.dataset.id);
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' });
    this.refresh();
  },
  goStylist(e) { wx.navigateTo({ url: '/pages/stylist/stylist?id=' + e.currentTarget.dataset.id }); },
  book(e) { wx.navigateTo({ url: '/pages/booking/booking?stylistId=' + e.currentTarget.dataset.id }); },
  goAll() { this.setData({ filter: 'all' }); this.refresh(); }
});
