const data = require('../../utils/spots.js');
const store = require('../../utils/store.js');
const WALKER = 999;

Page({
  data: { stops: [], markers: [], polyline: [], points: [], lat: 39.913, lng: 116.397, total: '0 米', walk: '0 分钟', playing: false },
  onReady() {
    this.map = wx.createMapContext('routeMap', this);
  },
  onShow() {
    if (this.getTabBar && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    this.refresh();
  },
  refresh() {
    const ids = store.get('route', []);
    const stops = ids.map(data.byId).filter(Boolean).map((s, i, a) => Object.assign({}, s, {
      leg: i > 0 ? data.fmtDistance(data.distance(a[i - 1], s)) : ''
    }));
    let m = 0;
    for (let i = 1; i < stops.length; i++) m += data.distance(stops[i - 1], stops[i]);
    const pts = stops.map((s) => ({ latitude: s.lat, longitude: s.lng }));
    const markers = data.markers(stops, 0).map((mk, i) => Object.assign(mk, { label: { content: String(i + 1), color: '#ffffff', bgColor: '#1c3d5a', fontSize: 11, padding: 3, borderRadius: 9, anchorX: -6, anchorY: -46 } }));
    this.setData({
      stops: stops,
      markers: markers,
      polyline: pts.length >= 2 ? [{ points: pts, color: '#1c3d5add', width: 6, arrowLine: true }] : [],
      points: pts,
      total: data.fmtDistance(m),
      // 步行按 4.5 公里 / 小时
      walk: Math.round(m / 75) + ' 分钟'
    });
  },
  move(from, to) {
    const ids = store.get('route', []);
    const [x] = ids.splice(from, 1);
    if (to >= 0) ids.splice(to, 0, x);
    store.set('route', ids);
    this.refresh();
  },
  up(e) { const i = e.currentTarget.dataset.index; this.move(i, i - 1); },
  remove(e) {
    const s = this.data.stops[e.currentTarget.dataset.index];
    wx.showModal({ title: '移除地点', content: '把「' + s.name + '」从路线里拿掉？', success: (r) => { if (r.confirm) this.move(e.currentTarget.dataset.index, -1); } });
  },
  open(e) { wx.navigateTo({ url: '/pages/spot/spot?id=' + e.currentTarget.dataset.id }); },
  onMarker(e) {
    if (e.detail.markerId !== WALKER) wx.navigateTo({ url: '/pages/spot/spot?id=' + e.detail.markerId });
  },
  fit() {
    if (this.map) this.map.includePoints({ points: this.data.points, padding: [40, 40, 40, 40] });
  },
  play() {
    if (this.data.playing || !this.map) return;
    const pts = this.data.points;
    // 一个小人沿路线走一遍（moveAlong），走完自动收起
    const walker = { id: WALKER, latitude: pts[0].latitude, longitude: pts[0].longitude, width: 24, height: 24, iconPath: '/images/pin/walker.png', anchor: { x: 0.5, y: 0.5 } };
    this.setData({ playing: true });
    this.map.addMarkers({
      markers: [walker],
      success: () => this.map.moveAlong({
        markerId: WALKER, path: pts, duration: 1500 * pts.length,
        success: () => setTimeout(() => { this.map.removeMarkers({ markerIds: [WALKER] }); this.setData({ playing: false }); }, 1500 * pts.length + 300),
        fail: () => this.setData({ playing: false })
      }),
      fail: (r) => { this.setData({ playing: false }); wx.showToast({ title: '当前环境不支持动画', icon: 'none' }); }
    });
  }
});
