const util = require('../../utils/util.js');
const { STORES, DEFAULT_LOC } = require('../../utils/data.js');

// markers 的 id 必须是 number：用门店下标 + 1
function markersOf(list, activeId) {
  return list.map((s, i) => ({
    id: i + 1, latitude: s.latitude, longitude: s.longitude, width: 30, height: 30,
    title: s.short, iconPath: '/images/svc/oil.png',
    callout: { content: s.short + ' · ' + s.dist + 'km', display: s.id === activeId ? 'ALWAYS' : 'BYCLICK', padding: 6, borderRadius: 6, fontSize: 12, bgColor: '#ffffff', color: '#1d2433' }
  }));
}

Page({
  data: {
    loc: DEFAULT_LOC, list: [], markers: [], points: [], activeId: '', sort: 'dist', loading: true, locTip: ''
  },
  onLoad() {
    this.map = wx.createMapContext('storeMap', this);
  },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 1 });
    util.getLocation((loc) => {
      const list = STORES.map((s) => Object.assign({}, s, { dist: util.distanceKm(loc, s) }));
      this.setData({
        loc, loading: false, locTip: loc.ok ? '' : '定位不可用，已按「' + loc.name + '」计算距离',
        points: list.map((s) => ({ latitude: s.latitude, longitude: s.longitude })).concat([{ latitude: loc.latitude, longitude: loc.longitude }])
      });
      this.applySort(list, this.data.sort);
    });
  },
  applySort(list, sort) {
    list = list.slice().sort((a, b) => (sort === 'dist' ? a.dist - b.dist : b.rating - a.rating));
    this.setData({ list, sort, markers: markersOf(list, this.data.activeId) });
  },
  onSort(e) { this.applySort(this.data.list, e.currentTarget.dataset.k); },
  onMarkerTap(e) {
    const s = this.data.list[e.detail.markerId - 1];
    if (!s) return;
    this.setData({ activeId: s.id, markers: markersOf(this.data.list, s.id) });
    wx.showToast({ title: s.short + '，距你 ' + s.dist + ' km', icon: 'none' });
  },
  // 回到我的位置：宿主地图没接入时 moveToLocation 走 fail，改成直接把中心点设回定位坐标
  locate() {
    this.map.moveToLocation({
      fail: () => {
        this.setData({ loc: Object.assign({}, this.data.loc) });
        wx.showToast({ title: '已回到' + (this.data.loc.ok ? '当前位置' : this.data.loc.name), icon: 'none' });
      }
    });
  },
  // 选择取送车 / 出发位置，重新按它排序
  chooseFrom() {
    wx.chooseLocation({
      latitude: this.data.loc.latitude, longitude: this.data.loc.longitude,
      success: (r) => {
        const loc = { latitude: r.latitude, longitude: r.longitude, name: r.name || r.address, ok: true };
        const list = STORES.map((s) => Object.assign({}, s, { dist: util.distanceKm(loc, s) }));
        this.setData({ loc, locTip: '' });
        this.applySort(list, this.data.sort);
      },
      fail: () => wx.showToast({ title: '暂不能选点，仍按「' + this.data.loc.name + '」排序', icon: 'none' })
    });
  },
  pick(e) { this.setData({ activeId: e.currentTarget.dataset.id, markers: markersOf(this.data.list, e.currentTarget.dataset.id) }); },
  goStore(e) { wx.navigateTo({ url: '/pages/store/store?id=' + e.currentTarget.dataset.id }); },
  call(e) {
    const s = this.data.list.find((x) => x.id === e.currentTarget.dataset.id);
    util.callPhone(s.phone, s.name);
  },
  nav(e) { util.openStoreLocation(this.data.list.find((x) => x.id === e.currentTarget.dataset.id)); }
});
