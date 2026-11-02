const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { tab: 'fav', list: [], favN: 0, viewN: 0 },
  onLoad(q) { if (q.tab === 'view') this.setData({ tab: 'view' }); },
  onShow() { this.load(); },
  load() {
    const favs = store.favs();
    const ids = this.data.tab === 'fav' ? favs : store.views();
    this.setData({
      favN: favs.length, viewN: store.views().length,
      list: ids.map(data.hotelOf).map(h => ({ id: h.id, name: h.name, img: h.img, score: h.score, city: h.city, area: h.area, starText: h.starText, price: h.price, fav: favs.indexOf(h.id) >= 0 }))
    });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.k }); this.load(); },
  onOpen(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  onFav(e) {
    const on = store.toggleFav(String(e.currentTarget.dataset.id));
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' });
    this.load();
  },
  onClear() {
    wx.showModal({
      title: '清空浏览记录', content: '最近看过的 ' + this.data.viewN + ' 家酒店将被清空', confirmText: '清空', confirmColor: '#FF5A36',
      success: r => { if (r.confirm) { store.clearViews(); this.load(); } }
    });
  },
  onGo() { wx.switchTab({ url: '/pages/index/index' }); }
});
