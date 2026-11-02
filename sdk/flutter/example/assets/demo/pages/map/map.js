// 地图演示：属性（标记 / 气泡 / 标签 / 折线 / 圆 / 多边形 / 卫星图）、事件、MapContext。
// 坐标是 GCJ-02（微信 map 的默认坐标系）。带 ?auto=1 打开时按顺序把 MapContext 的方法跑一遍，
// 每一步打一行 `NC-CHECK map …`（tools/android-native-components.sh / ios 同名脚本据此判通过）。
const HOME = { latitude: 39.908823, longitude: 116.39747 };
const PLACES = [
  { id: 1, name: '天安门', latitude: 39.908823, longitude: 116.39747 },
  { id: 2, name: '故宫博物院', latitude: 39.916345, longitude: 116.397155 },
  { id: 3, name: '王府井', latitude: 39.914612, longitude: 116.410902 },
  { id: 4, name: '国家大剧院', latitude: 39.903527, longitude: 116.388893 },
  { id: 5, name: '前门大街', latitude: 39.899164, longitude: 116.398024 }
];

function markers() {
  return PLACES.map((p, i) => {
    const m = { id: p.id, latitude: p.latitude, longitude: p.longitude, title: p.name, width: 28, height: 36 };
    if (i === 0) m.callout = { content: '天安门 · 起点', display: 'ALWAYS', padding: 8, borderRadius: 6, bgColor: '#07c160', color: '#ffffff', fontSize: 13 };
    else if (i === 3) {
      m.iconPath = '/images/avatar.jpg';
      m.width = 32;
      m.height = 32;
      m.label = { content: p.name, color: '#333333', fontSize: 12, anchorX: -30, anchorY: 4, bgColor: '#ffffffdd', padding: 4, borderRadius: 4 };
    } else m.callout = { content: p.name + "（点我）", display: 'BYCLICK', padding: 6, borderRadius: 4, fontSize: 12 };
    return m;
  });
}

const ROUTE = [{
  points: PLACES.slice(0, 3).map((p) => ({ latitude: p.latitude, longitude: p.longitude })),
  color: '#10aeffdd', width: 6, arrowLine: true
}, {
  points: [PLACES[0], PLACES[4]].map((p) => ({ latitude: p.latitude, longitude: p.longitude })),
  color: '#fa9d3bdd', width: 4, dottedLine: true
}];

Page({
  data: {
    latitude: HOME.latitude,
    longitude: HOME.longitude,
    scale: 14,
    satellite: false,
    traffic: false,
    markers: markers(),
    polyline: ROUTE,
    circles: [{ latitude: HOME.latitude, longitude: HOME.longitude, radius: 600, color: '#07c160aa', fillColor: '#07c16022', strokeWidth: 2 }],
    polygons: [{
      points: [
        { latitude: 39.9048, longitude: 116.3873 }, { latitude: 39.9048, longitude: 116.3906 },
        { latitude: 39.9022, longitude: 116.3906 }, { latitude: 39.9022, longitude: 116.3873 }
      ],
      strokeWidth: 2, strokeColor: '#8a5cf5cc', fillColor: '#8a5cf533'
    }],
    centerText: HOME.latitude.toFixed(5) + ', ' + HOME.longitude.toFixed(5),
    scaleText: '14',
    ctxResult: '点上面的按钮调 MapContext',
    logs: []
  },
  onLoad(q) {
    this.auto = q && q.auto === '1';
    // chain=1：三个自测串着跑（地图 → web-view → 直播），真机脚本一次启动测完
    this.chain = q && q.chain === '1';
  },
  onReady() {
    this.ctx = wx.createMapContext('map', this);
    if (this.auto) setTimeout(() => this.runAuto(), 1500);
  },
  log(s) {
    const t = new Date();
    const line = t.getMinutes() + ':' + ('0' + t.getSeconds()).slice(-2) + ' ' + s;
    this.setData({ logs: [line].concat(this.data.logs).slice(0, 12) });
    console.log('[map] ' + s);
  },
  call(name, opt) {
    return new Promise((resolve) => {
      this.ctx[name](Object.assign({}, opt || {}, {
        success: (r) => { this.setData({ ctxResult: name + ' ✓ ' + JSON.stringify(r) }); resolve(r); },
        fail: (r) => { this.setData({ ctxResult: name + ' ✗ ' + r.errMsg }); resolve(null); }
      }));
    });
  },
  center() { this.call('getCenterLocation'); },
  region() { this.call('getRegion'); },
  getScale() { this.call('getScale'); },
  moveHome() { this.call('moveToLocation', HOME); },
  include() { this.call('includePoints', { points: PLACES, padding: [40, 40, 40, 40] }); },
  translate() {
    this.call('translateMarker', {
      markerId: 5, destination: { latitude: 39.9055, longitude: 116.4040 }, autoRotate: false, duration: 1000,
      animationEnd: () => this.log('translateMarker animationEnd')
    });
  },
  addMarker() {
    this.call('addMarkers', { markers: [{ id: 99, latitude: 39.9123, longitude: 116.4040, title: '新加的标记', callout: { content: '新加的标记', display: 'ALWAYS' } }] });
  },
  removeMarker() { this.call('removeMarkers', { markerIds: [99] }); },
  toScreen() { this.call('toScreenLocation', HOME); },
  zoom(e) {
    const s = Math.max(5, Math.min(19, this.data.scale + Number(e.currentTarget.dataset.d)));
    this.setData({ scale: s, scaleText: String(s) });
  },
  toggle(e) {
    const k = e.currentTarget.dataset.k;
    this.setData({ [k]: !this.data[k] });
  },
  toggleRoute() { this.setData({ polyline: this.data.polyline.length ? [] : ROUTE }); },
  onTap(e) { this.log('tap ' + e.detail.latitude.toFixed(5) + ', ' + e.detail.longitude.toFixed(5)); },
  onMarkerTap(e) { this.log('markertap markerId=' + e.detail.markerId + '（' + typeof e.detail.markerId + '）'); },
  onCalloutTap(e) { this.log('callouttap markerId=' + e.detail.markerId); },
  onLabelTap(e) { this.log('labeltap markerId=' + e.detail.markerId); },
  onUpdated() { if (!this.updated) { this.updated = true; this.log('updated（地图画好了）'); } },
  onRegion(e) {
    // 微信：detail 里有 type（begin / end）与 causedBy；老版本放在事件对象上，两处都认
    const d = e.detail || {};
    const type = d.type || e.type;
    if (type === 'end') {
      const c = d.centerLocation || {};
      this.setData({
        centerText: (c.latitude || 0).toFixed(5) + ', ' + (c.longitude || 0).toFixed(5),
        scaleText: String(Math.round((d.scale || 0) * 10) / 10)
      });
    }
    this.log('regionchange ' + type + ' causedBy=' + (d.causedBy || e.causedBy || '-'));
  },
  async runAuto() {
    const r = {};
    const c = await this.call('getCenterLocation');
    r.center = !!(c && Math.abs(c.latitude - HOME.latitude) < 0.01);
    const s = await this.call('getScale');
    r.scale = s ? s.scale : null;
    await this.call('moveToLocation', { latitude: PLACES[2].latitude, longitude: PLACES[2].longitude });
    await new Promise((ok) => setTimeout(ok, 800));
    const c2 = await this.call('getCenterLocation');
    r.moved = !!(c2 && Math.abs(c2.longitude - PLACES[2].longitude) < 0.002);
    const g = await this.call('getRegion');
    r.region = !!(g && g.southwest && g.northeast && g.northeast.latitude > g.southwest.latitude);
    r.include = !!(await this.call('includePoints', { points: PLACES, padding: [30, 30, 30, 30] }));
    const pt = await this.call('toScreenLocation', HOME);
    r.screen = !!(pt && typeof pt.x === 'number');
    r.translate = !!(await this.call('translateMarker', { markerId: 5, destination: { latitude: 39.9055, longitude: 116.404 }, duration: 300, autoRotate: false }));
    r.ok = r.center && r.moved && r.region && r.include && r.screen && r.translate;
    console.log('NC-CHECK map ' + JSON.stringify(r));
    if (this.chain) wx.redirectTo({ url: '/pages/webview/webview?auto=1&chain=1' });
  }
});
