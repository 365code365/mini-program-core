// 展厅列表：楼层筛选 + scroll-view 自定义下拉刷新（refresher-enabled / bindrefresherrefresh / refresher-triggered）。
// 下拉刷新拉的是各展厅的实时客流（馆内闸机计数），刷新完把 refresher-triggered 设回 false 收起指示器
const { HALLS } = require('../../utils/data.js');
const U = require('../../utils/util.js');

const FLOORS = ['全部', '一层', '二层', '三层'];
const CAP = { h1: 600, h2: 650, h3: 300, h4: 420, h5: 520 };

function crowd(seed) {
  const now = new Date();
  return HALLS.map((h, i) => {
    const base = (now.getHours() * 37 + i * 53 + seed * 17) % 100;
    const n = Math.round(CAP[h.id] * (0.15 + base / 130));
    const r = n / CAP[h.id];
    return { id: h.id, n, level: r < 0.45 ? '舒适' : r < 0.75 ? '适中' : '较拥挤', lv: r < 0.45 ? 0 : r < 0.75 ? 1 : 2 };
  });
}

Page({
  data: { floors: FLOORS, floor: 0, list: [], triggered: false, updated: '', seed: 0, svH: 500 },
  onLoad() {
    const win = wx.getWindowInfo();
    // scroll-view 要定高：窗口高减去楼层筛选条（44px）
    this.setData({ svH: win.windowHeight - 44 });
    this.refresh(0);
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
  },
  refresh(seed) {
    const c = crowd(seed);
    const fav = U.load('fav', []);
    const f = FLOORS[this.data.floor];
    const list = HALLS.filter(h => f === '全部' || h.floor === f).map(h => {
      const k = c.find(x => x.id === h.id);
      return Object.assign({}, h, { crowd: k, favN: h.exhibits.filter(id => fav.indexOf(id) >= 0).length });
    });
    const d = new Date();
    this.setData({ list, seed, updated: U.pad2(d.getHours()) + ':' + U.pad2(d.getMinutes()) });
  },
  onFloor(e) {
    this.setData({ floor: +e.currentTarget.dataset.i });
    this.refresh(this.data.seed);
  },
  onRefresh() {
    // 模拟客流接口 600ms 返回
    setTimeout(() => {
      this.refresh(this.data.seed + 1);
      this.setData({ triggered: false });
      wx.showToast({ title: '已更新实时客流', icon: 'none' });
    }, 600);
  },
  openHall(e) { wx.navigateTo({ url: '/pages/hall/hall?id=' + e.currentTarget.dataset.id }); }
});
