// 游览路线：map 画 polyline（路线）+ markers（站点）+ callout（站点说明）；点站点列表 / marker 联动。
// 桌面端没有地图宿主时 map 只画占位，下方站点列表照常可用。
const data = require('../../utils/data.js');
const app = getApp();

Page({
  data: { s: null, ri: 0, route: null, markers: [], polyline: [], lat: 0, lng: 0, scale: 13, sel: 0, located: '' },
  onLoad(q) {
    const s = data.find((q && q.id) || app.globalData.scenic);
    wx.setNavigationBarTitle({ title: s.short + ' · 游览路线' });
    this.setData({ s: { id: s.id, name: s.name, routes: s.routes } });
    this.build(0, 0);
    this.locate();
  },
  build(ri, sel) {
    const route = this.data.s.routes[ri];
    const stops = route.stops.map((p, i) => ({ i, name: p[0], lat: p[1], lng: p[2], note: p[3] }));
    const markers = stops.map((p) => ({
      id: p.i, latitude: p.lat, longitude: p.lng, title: p.name, width: 26, height: 34,
      label: { content: String(p.i + 1), color: '#ffffff', fontSize: 11, anchorX: -4, anchorY: -28 },
      callout: { content: (p.i + 1) + '. ' + p.name + '\n' + p.note, color: '#1d2722', fontSize: 12, borderRadius: 8, padding: 8,
        bgColor: '#ffffff', display: p.i === sel ? 'ALWAYS' : 'BYCLICK' }
    }));
    const polyline = [{ points: stops.map((p) => ({ latitude: p.lat, longitude: p.lng })), color: '#13a07bDD', width: 6, arrowLine: true, borderColor: '#ffffff', borderWidth: 2 }];
    const lats = stops.map((p) => p.lat), lngs = stops.map((p) => p.lng);
    const span = Math.max(Math.max(...lats) - Math.min(...lats), Math.max(...lngs) - Math.min(...lngs));
    this.setData({
      ri, sel, route: { name: route.name, time: route.time, stops }, markers, polyline,
      lat: (Math.max(...lats) + Math.min(...lats)) / 2, lng: (Math.max(...lngs) + Math.min(...lngs)) / 2,
      scale: span > 0.15 ? 11 : span > 0.04 ? 13 : 15
    });
  },
  locate() {
    wx.getLocation({
      type: 'gcj02',
      success: (r) => {
        const p = this.data.route.stops[0];
        const km = Math.sqrt(Math.pow((r.latitude - p.lat) * 111, 2) + Math.pow((r.longitude - p.lng) * 111 * Math.cos(p.lat * Math.PI / 180), 2));
        this.setData({ located: '你距「' + p.name + '」约 ' + (km < 10 ? km.toFixed(1) : Math.round(km)) + ' 公里' });
      },
      fail: () => this.setData({ located: '未获取到定位，按景区入口规划路线' })
    });
  },
  onRoute(e) { const ri = +e.currentTarget.dataset.i; if (ri !== this.data.ri) this.build(ri, 0); },
  select(i) {
    if (i === this.data.sel) return;
    const p = this.data.route.stops[i];
    const markers = this.data.markers.map((m) => Object.assign({}, m, { callout: Object.assign({}, m.callout, { display: m.id === i ? 'ALWAYS' : 'BYCLICK' }) }));
    this.setData({ sel: i, markers, lat: p.lat, lng: p.lng });
  },
  onStop(e) { this.select(+e.currentTarget.dataset.i); },
  onMarker(e) { this.select(e.detail.markerId); },
  onCallout(e) { this.select(e.detail.markerId); },
  onCopy() {
    const r = this.data.route;
    wx.setClipboardData({ data: this.data.s.name + '「' + r.name + '」' + r.time + '：' + r.stops.map((p) => p.name).join(' → ') });
  },
  goGuide() { app.setScenic(this.data.s.id); wx.switchTab({ url: '/pages/guide/guide' }); }
});
