const { OILS, stationById, TREND, DEFAULT_LOC } = require('../../utils/data.js');
const geo = require('../../utils/geo.js');
const nav = require('../../utils/nav.js');

const FAC = [['便利店', '🏪'], ['洗车', '🚿'], ['卫生间', '🚻'], ['免费充气', '🛞'], ['咖啡', '☕'], ['充电桩', '🔌']];

Page({
  data: { st: {}, oil: '95#', prices: [], facilities: [], curOurs: '', distText: '', fallback: false, next: TREND.next, net: { isConnected: true } },
  onLoad(q) {
    const st = stationById(q.id) || stationById('s1');
    this.st = st;
    const oil = st.prices[decodeURIComponent(q.oil || '')] ? decodeURIComponent(q.oil) : Object.keys(st.prices)[0];
    wx.setNavigationBarTitle({ title: st.name.replace(/加油站$/, '') });
    this.setData({
      st, oil,
      prices: OILS.filter((o) => st.prices[o]).map((o) => ({ oil: o, list: st.prices[o].list.toFixed(2), ours: st.prices[o].ours.toFixed(2), cut: (st.prices[o].list - st.prices[o].ours).toFixed(2) })),
      facilities: FAC.map(([name, ic]) => ({ name, ic, on: st.facilities.indexOf(name) >= 0 }))
    });
    this.applyOil(oil);
    geo.locate((loc) => this.setData({ distText: geo.fmtDistance(geo.distance(loc.latitude, loc.longitude, st.lat, st.lng)), fallback: loc.fallback }));
  },
  onShow() { this.onNetChange(getApp().globalData.net); },
  onNetChange(n) { this.setData({ net: n }); },
  pickOil(e) { this.applyOil(e.currentTarget.dataset.oil); },
  applyOil(oil) { this.setData({ oil, curOurs: this.st.prices[oil].ours.toFixed(2) }); },
  navigate() { nav.openStation(this.st); },
  call() {
    wx.makePhoneCall({ phoneNumber: this.st.phone, fail: () => wx.showModal({ title: this.st.name, content: '油站电话：' + this.st.phone, confirmText: '复制号码', success: (r) => r.confirm && wx.setClipboardData({ data: this.st.phone }) }) });
  },
  refuel() { wx.navigateTo({ url: `/pages/refuel/refuel?id=${this.st.id}&oil=${encodeURIComponent(this.data.oil)}` }); },
  onShareAppMessage() { return { title: `${this.st.name} ${this.data.oil} 到站价 ¥${this.data.curOurs}`, path: `/pages/station/station?id=${this.st.id}` }; }
});
