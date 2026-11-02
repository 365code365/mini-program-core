const { OILS, stations, stationById } = require('../../utils/data.js');
const geo = require('../../utils/geo.js');
const store = require('../../utils/store.js');
const nav = require('../../utils/nav.js');

const NET = { wifi: 'Wi-Fi', '2g': '2G', '3g': '3G', '4g': '4G', '5g': '5G', none: '无网络', unknown: '未知网络' };

Page({
  data: {
    net: { isConnected: true, networkType: 'wifi' }, netText: '', loc: { name: '定位中…' }, oils: OILS, oilIndex: 1, sort: 'distance', mode: 'list',
    list: [], markers: [], activeId: '', intoView: ''
  },

  onLoad() {
    this.setData({ oilIndex: Math.max(0, OILS.indexOf(store.oilPref())) });
    this.relocate();
  },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 0 });
    this.onNetChange(getApp().globalData.net);
    // 「我的」里改了油号偏好，回来同步
    const i = OILS.indexOf(store.oilPref());
    if (i >= 0 && i !== this.data.oilIndex) { this.setData({ oilIndex: i }); this.build(); }
  },
  onNetChange(n) { this.setData({ net: n, netText: NET[n.networkType] || n.networkType }); },

  relocate() {
    this.setData({ 'loc.name': '定位中…' });
    geo.locate((loc) => {
      this.loc = loc;
      this.setData({ loc });
      if (loc.fallback) wx.showToast({ title: '定位失败，已按默认位置展示', icon: 'none' });
      this.build();
    });
  },

  // 只列供应所选油号的站，haversine 算距离；价格排序用到站价，同价按距离
  build() {
    if (!this.loc) return;
    const oil = OILS[this.data.oilIndex];
    const rows = stations.filter((s) => s.prices[oil]).map((s) => {
      const d = geo.distance(this.loc.latitude, this.loc.longitude, s.lat, s.lng);
      const pr = s.prices[oil];
      return {
        id: s.id, name: s.name, short: s.name.replace(/^(中国石化|中国石油|中化石油|壳牌|易加油)/, ''), logo: s.logo, address: s.address, hours: s.hours, rating: s.rating,
        lat: s.lat, lng: s.lng, dist: d, distText: geo.fmtDistance(d), ours: pr.ours.toFixed(2), listPrice: pr.list.toFixed(2), cut: (pr.list - pr.ours).toFixed(2)
      };
    }).filter((r) => r.dist <= 10000);
    const low = Math.min.apply(null, rows.map((r) => +r.ours));
    rows.forEach((r) => { r.cheapest = +r.ours === low; });
    rows.sort((a, b) => (this.data.sort === 'price' ? (+a.ours - +b.ours) || (a.dist - b.dist) : a.dist - b.dist));
    const markers = rows.map((r, i) => ({
      id: i + 1, latitude: r.lat, longitude: r.lng, iconPath: r.logo, width: 30, height: 30, title: r.name,
      label: { content: `¥${r.ours}`, color: '#ffffff', fontSize: 12, bgColor: r.cheapest ? '#f2542d' : '#13a874', borderRadius: 4, padding: 3, anchorX: -18, anchorY: -52 },
      callout: { content: `${r.short}\n${oil} ¥${r.ours} · ${r.distText}`, color: '#1d2329', fontSize: 12, bgColor: '#ffffff', borderRadius: 6, padding: 6, display: 'BYCLICK' }
    }));
    this.setData({ list: rows, markers, activeId: this.data.activeId || (rows[0] && rows[0].id) || '' });
  },

  onOil(e) {
    const i = Number(e.detail.value);
    this.setData({ oilIndex: i });
    store.setOilPref(OILS[i]);
    this.build();
  },
  setSort(e) { this.setData({ sort: e.currentTarget.dataset.s }); this.build(); },
  toggleMode() { this.setData({ mode: this.data.mode === 'list' ? 'map' : 'list' }); },
  onMarker(e) {
    const r = this.data.list[Number(e.detail.markerId) - 1];
    if (r) this.setData({ activeId: r.id, intoView: 'mc-' + r.id });
  },
  onRegion() {},
  pickCard(e) {
    const id = e.currentTarget.dataset.id;
    if (this.data.activeId === id) { this.openStation(e); return; }
    this.setData({ activeId: id });
  },
  openStation(e) { wx.navigateTo({ url: `/pages/station/station?id=${e.currentTarget.dataset.id}&oil=${encodeURIComponent(OILS[this.data.oilIndex])}` }); },
  refuel(e) { wx.navigateTo({ url: `/pages/refuel/refuel?id=${e.currentTarget.dataset.id}&oil=${encodeURIComponent(OILS[this.data.oilIndex])}` }); },
  navigate(e) { nav.openStation(stationById(e.currentTarget.dataset.id)); },
  goCoupons() { wx.navigateTo({ url: '/pages/cards/cards?tab=coupon' }); },
  onShareAppMessage() { return { title: '附近油站比价，到站直降 0.55 元/升', path: '/pages/index/index' }; }
});
