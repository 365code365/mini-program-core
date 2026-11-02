const data = require('../../utils/spots.js');
const store = require('../../utils/store.js');

Page({
  data: { favs: [], shots: [], trail: [] },
  onShow() {
    if (this.getTabBar && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    this.setData({
      favs: store.get('fav', []).map(data.byId).filter(Boolean),
      shots: store.get('shots', []),
      trail: store.get('trail', []).map((t) => Object.assign({ name: (data.byId(t.id) || {}).name || '' }, t))
    });
  },
  open(e) { wx.navigateTo({ url: '/pages/spot/spot?id=' + e.currentTarget.dataset.id }); },
  preview(e) {
    const urls = this.data.shots.map((s) => s.path);
    wx.previewImage({ urls: urls, current: urls[e.currentTarget.dataset.index] });
  },
  clearTrail() {
    wx.showModal({ title: '清空足迹', content: '清空后不能恢复', success: (r) => { if (r.confirm) { store.set('trail', []); this.setData({ trail: [] }); } } });
  }
});
