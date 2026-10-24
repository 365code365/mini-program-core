const data = require('../../utils/data.js');

Page({
  data: {
    stations: data.stations,
    list: [],
    selId: '',
    keyword: '',
    filters: ['距离最近', '有快充空闲', '价格最低', '24 小时', '停车免费'],
    fIdx: 0,
    session: null
  },
  onLoad() { this.refresh(); },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    this.setData({ session: wx.getStorageSync('ev_session') || null });
  },
  refresh() {
    const kw = this.data.keyword.trim();
    let list = data.stations.slice();
    if (kw) list = list.filter(s => (s.name + s.addr).indexOf(kw) >= 0);
    const f = this.data.fIdx;
    if (f === 1) list = list.filter(s => s.fastFree > 0);
    if (f === 3) list = list.filter(s => s.open === '24 小时');
    if (f === 4) list = list.filter(s => s.parking.indexOf('免费') >= 0);
    if (f === 2) list.sort((a, b) => a.price - b.price);
    else list.sort((a, b) => a.dist - b.dist);
    this.setData({ list });
  },
  onSearch(e) { this.setData({ keyword: e.detail.value }); this.refresh(); },
  onSearchTap() {},
  onFilter(e) { this.setData({ fIdx: e.currentTarget.dataset.idx }); this.refresh(); },
  onMarker(e) {
    const id = e.currentTarget.dataset.id;
    // 选中的电站排到列表最前，像地图 App 一样联动
    const list = this.data.list.slice().sort((a, b) => (a.id === id ? -1 : b.id === id ? 1 : 0));
    this.setData({ selId: id, list });
    const s = data.getStation(id);
    wx.showToast({ title: s.name.split('·').pop() + ' ' + s.distText, icon: 'none' });
  },
  onLocate() { this.setData({ selId: '' }); this.refresh(); wx.showToast({ title: '已定位到当前位置', icon: 'none' }); },
  onScan() { wx.navigateTo({ url: '/pages/scan/scan' }); },
  onSession() { wx.navigateTo({ url: '/pages/charging/charging' }); },
  onStation(e) { wx.navigateTo({ url: '/pages/station/station?id=' + e.currentTarget.dataset.id }); }
});
