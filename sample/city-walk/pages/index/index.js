const data = require('../../utils/spots.js');
const store = require('../../utils/store.js');
const CENTER = { lat: 39.913, lng: 116.397 };

Page({
  data: {
    cats: data.CATS, cat: 'all', list: [], markers: [], lat: CENTER.lat, lng: CENTER.lng, scale: 13,
    current: null, currentDist: '', viewHint: '拖动地图看看周边'
  },
  onLoad() {
    this.apply('all', 0);
  },
  onReady() {
    this.map = wx.createMapContext('cityMap', this);
  },
  onShow() {
    if (this.getTabBar && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
  },
  apply(cat, selected) {
    const colors = {};
    data.CATS.forEach((c) => { colors[c.key] = c.color; });
    const list = data.SPOTS.filter((s) => cat === 'all' || s.cat === cat).map((s) => Object.assign({ color: colors[s.cat] }, s));
    this.setData({ cat: cat, list: list, markers: data.markers(list, selected) });
  },
  pickCat(e) {
    this.apply(e.currentTarget.dataset.key, this.data.current ? this.data.current.id : 0);
  },
  select(id, move) {
    const s = data.byId(id);
    if (!s) return;
    this.setData({ current: s, markers: data.markers(this.data.list, s.id) });
    if (move) this.setData({ lat: s.lat, lng: s.lng, scale: 15 });
    this.updateDist();
  },
  onMarker(e) {
    this.select(e.detail.markerId, false);
  },
  focusSpot(e) {
    this.select(e.currentTarget.dataset.id, true);
    wx.pageScrollTo({ scrollTop: 0, duration: 200 });
  },
  onMapTap(e) {
    if (this.data.current) this.setData({ current: null, markers: data.markers(this.data.list, 0) });
  },
  onRegion(e) {
    const d = e.detail || {};
    if ((d.type || e.type) !== 'end' || !d.region) return;
    const r = d.region;
    const n = this.data.list.filter((s) => s.lat >= r.southwest.latitude && s.lat <= r.northeast.latitude &&
      s.lng >= r.southwest.longitude && s.lng <= r.northeast.longitude).length;
    this.center = d.centerLocation;
    this.setData({ viewHint: '当前视野内 ' + n + ' 个' });
    this.updateDist();
  },
  updateDist() {
    const s = this.data.current;
    const c = this.center || { latitude: this.data.lat, longitude: this.data.lng };
    if (!s) return;
    this.setData({ currentDist: data.fmtDistance(data.distance({ lat: c.latitude, lng: c.longitude }, s)) });
  },
  locate() {
    wx.getLocation({
      type: 'gcj02',
      success: (r) => this.setData({ lat: r.latitude, lng: r.longitude, scale: 15 }),
      fail: () => {
        // 宿主没给定位（或用户拒绝）：回到城市中心，告诉用户为什么
        wx.showToast({ title: '定位不可用，已回到市中心', icon: 'none' });
        if (this.map) this.map.moveToLocation({ latitude: CENTER.lat, longitude: CENTER.lng, fail: () => this.setData({ lat: CENTER.lat, lng: CENTER.lng }) });
        else this.setData({ lat: CENTER.lat, lng: CENTER.lng });
      }
    });
  },
  fitAll() {
    const pts = this.data.list.map((s) => ({ latitude: s.lat, longitude: s.lng }));
    if (!pts.length) return;
    if (!this.map) return;
    this.map.includePoints({ points: pts, padding: [40, 40, 40, 40], fail: () => this.setData({ scale: 12, lat: CENTER.lat, lng: CENTER.lng }) });
  },
  openSpot(e) {
    store.visit(Number(e.currentTarget.dataset.id));
    wx.navigateTo({ url: '/pages/spot/spot?id=' + e.currentTarget.dataset.id });
  }
});
