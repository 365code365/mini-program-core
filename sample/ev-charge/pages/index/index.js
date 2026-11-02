const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const FAV = 5;
Page({
  data: {
    stations: [],
    list: [],
    selId: '',
    keyword: '',
    filters: ['距离最近', '有快充空闲', '价格最低', '24 小时', '停车免费', '我的收藏'],
    fIdx: 0,
    session: null,
    unpaid: null,
    located: false
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    // 「我的 → 收藏电站」切过来时带着筛选（switchTab 不能带参数）
    const app = getApp();
    if (app.globalData.filter !== undefined) {
      this.setData({ fIdx: app.globalData.filter });
      delete app.globalData.filter;
    }
    const s = store.getSession();
    const u = store.unpaidOrder();
    this.setData({
      stations: store.stations(),
      session: s ? { terminal: s.no } : null,
      unpaid: u ? { id: u.id, pay: u.pay.toFixed(2) } : null
    });
    this.refresh();
  },
  refresh() {
    const kw = this.data.keyword.trim();
    let list = this.data.stations.slice();
    if (kw) list = list.filter(s => (s.name + s.addr).indexOf(kw) >= 0);
    const f = this.data.fIdx;
    if (f === 1) list = list.filter(s => s.fastFree > 0);
    if (f === 3) list = list.filter(s => s.open === '24 小时');
    if (f === 4) list = list.filter(s => s.parking.indexOf('免费') >= 0);
    if (f === FAV) {
      const favs = store.getFavs();
      list = list.filter(s => favs.indexOf(s.id) >= 0);
    }
    if (f === 2) list.sort((a, b) => a.price - b.price);
    else list.sort((a, b) => a.dist - b.dist);
    const id = this.data.selId;
    if (id) list.sort((a, b) => (a.id === id ? -1 : b.id === id ? 1 : 0));
    this.setData({ list, emptyText: f === FAV ? '还没有收藏电站，在电站详情点「收藏」' : '没有符合条件的电站' });
  },
  onSearch(e) { this.setData({ keyword: e.detail.value }); this.refresh(); },
  onFilter(e) { this.setData({ fIdx: Number(e.currentTarget.dataset.idx), selId: '' }); this.refresh(); },
  // 点地图上的电站：选中的电站排到列表最前，像地图 App 一样联动
  onMarker(e) {
    const id = String(e.currentTarget.dataset.id);
    this.setData({ selId: this.data.selId === id ? '' : id });
    this.refresh();
  },
  onLocate() {
    host.locate().then(ok => {
      this.setData({ selId: '', located: true });
      this.refresh();
      wx.showToast({ title: ok ? '已定位到当前位置' : '未开启定位，按「杭州·西溪」展示附近电站', icon: 'none' });
    });
  },
  onScan() { wx.navigateTo({ url: '/pages/scan/scan' }); },
  onSession() { wx.navigateTo({ url: '/pages/charging/charging' }); },
  onUnpaid() { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + this.data.unpaid.id }); },
  onStation(e) { wx.navigateTo({ url: '/pages/station/station?id=' + e.currentTarget.dataset.id }); },
  onNav(e) {
    const s = this.data.list.find(x => x.id === String(e.currentTarget.dataset.id));
    if (s) host.openMap(s);
  }
});
