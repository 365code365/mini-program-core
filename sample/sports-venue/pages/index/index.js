const D = require('../../utils/data.js');
const F = require('../../utils/fmt.js');

const AREAS = [
  { name: '杭州 · 西湖区', latitude: 30.2779, longitude: 120.1397 },
  { name: '杭州 · 滨江区', latitude: 30.2084, longitude: 120.2119 },
  { name: '杭州 · 上城区', latitude: 30.2425, longitude: 120.1697 },
  { name: '杭州 · 拱墅区', latitude: 30.3195, longitude: 120.1415 }
];

Page({
  data: {
    sports: D.SPORTS, sport: '', sportName: '', sort: 'dist', list: [],
    locName: '定位中…', locTip: '', loading: true, updated: ''
  },
  onLoad() { this.locate(); },
  onShow() { this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 0 }); },
  locate() {
    this.setData({ locName: '定位中…', loading: true });
    wx.getLocation({
      type: 'gcj02',
      success: (r) => this.useLoc({ latitude: r.latitude, longitude: r.longitude, name: '当前位置' }, ''),
      // 没有定位权限 / 桌面没有定位：退回默认区域，照样能按距离排序
      fail: () => this.useLoc(D.DEFAULT_LOC, '未获取到定位，已按西湖区为你推荐')
    });
  },
  useLoc(loc, tip) {
    getApp().globalData.loc = loc;
    this.loc = loc;
    this.setData({ locName: loc.name, locTip: tip });
    this.load();
  },
  chooseArea() {
    wx.showActionSheet({
      itemList: ['重新定位'].concat(AREAS.map((a) => a.name)),
      success: (r) => {
        if (r.tapIndex === 0) this.locate();
        else this.useLoc(AREAS[r.tapIndex - 1], '');
      }
    });
  },
  load(cb) {
    const loc = this.loc || D.DEFAULT_LOC;
    let list = D.VENUES.filter((v) => !this.data.sport || v.sport === this.data.sport).map((v) => {
      const dist = D.distance(loc, { latitude: v.lat, longitude: v.lng });
      return {
        id: v.id, name: v.name, photo: v.photo, addr: v.addr, rating: v.rating, dist, distText: F.km(dist),
        price: v.price, sport: D.sportName(v.sport), tags: v.facilities.slice(0, 2), open: v.open
      };
    });
    list.sort(this.data.sort === 'dist' ? (a, b) => a.dist - b.dist : (a, b) => b.rating - a.rating);
    const d = new Date();
    this.setData({ list, loading: false, updated: F.pad(d.getHours()) + ':' + F.pad(d.getMinutes()) });
    cb && cb();
  },
  onPullDownRefresh() {
    this.load(() => { wx.stopPullDownRefresh(); wx.showToast({ title: '空场已刷新', icon: 'none' }); });
  },
  refresh() { wx.startPullDownRefresh(); },
  pickSport(e) {
    const id = e.currentTarget.dataset.id;
    const sport = this.data.sport === id ? '' : id;
    this.setData({ sport, sportName: D.sportName(sport) });
    this.load();
    wx.pageScrollTo({ selector: '#nearby', duration: 300 });
  },
  clearSport() { this.setData({ sport: '', sportName: '' }); this.load(); },
  toggleSort(e) { this.setData({ sort: e.currentTarget.dataset.s }); this.load(); },
  toNearby() { wx.pageScrollTo({ selector: '#nearby', duration: 300 }); },
  open(e) { wx.navigateTo({ url: '/pages/venue/venue?id=' + e.currentTarget.dataset.id }); },
  onShareAppMessage() { return { title: '约场：附近球馆空场一键订', path: '/pages/index/index' }; }
});
