const D = require('../../utils/date');
const C = require('../../utils/cities');
const F = require('../../utils/flights');
const store = require('../../utils/store');
const app = getApp();

const CABINS = ['经济舱', '公务舱 / 头等舱'];
const DEALS = [
  { code: 'SYX', img: '/images/dest/sanya.jpg', tip: '椰风海韵' },
  { code: 'CTU', img: '/images/dest/chengdu.jpg', tip: '熊猫故乡' },
  { code: 'SIA', img: '/images/dest/xian.jpg', tip: '十三朝古都' },
  { code: 'KMG', img: '/images/dest/kunming.jpg', tip: '四季如春' }
];

Page({
  data: {
    trip: 'one', from: null, to: null, dep: '', ret: '',
    depMd: '', depWk: '', retMd: '', retWk: '', nights: 0,
    cabin: 0, cabinName: CABINS[0], history: [], deals: []
  },
  onLoad() {
    const last = wx.getStorageSync('fl_last') || {};
    const tomorrow = D.addDays(D.today(), 1);
    let dep = last.dep && D.diffDays(D.today(), last.dep) >= 0 ? last.dep : tomorrow;
    let ret = last.ret && D.diffDays(dep, last.ret) > 0 ? last.ret : D.addDays(dep, 3);
    this.setData({
      trip: last.trip || 'one', cabin: last.cabin || 0, cabinName: CABINS[last.cabin || 0],
      from: this.pick(C.getCity(last.from || 'HGH')), to: this.pick(C.getCity(last.to || 'BJS'))
    });
    this.setDates(dep, ret);
  },
  onShow() {
    app.syncTab(this, 0);
    this.setData({ history: store.history() });
    this.loadDeals();
  },
  pick(c) { return { code: c.code, name: c.name }; },
  setDates(dep, ret) {
    this.setData({
      dep, ret, depMd: D.md(dep), depWk: D.weekLabel(dep), retMd: D.md(ret), retWk: D.weekLabel(ret),
      nights: D.diffDays(dep, ret)
    });
    this.saveLast();
  },
  saveLast() {
    const d = this.data;
    if (!d.from || !d.to) return;
    wx.setStorageSync('fl_last', { trip: d.trip, from: d.from.code, to: d.to.code, dep: d.dep, ret: d.ret, cabin: d.cabin });
  },
  // 特价目的地：未来 30 天里的最低价
  loadDeals() {
    const from = this.data.from.code;
    const deals = DEALS.filter(x => x.code !== from).map(x => {
      let best = Infinity, bestDate = '';
      for (let i = 1; i <= 30; i++) {
        const d = D.addDays(D.today(), i), p = F.lowPrice(from, x.code, d);
        if (p < best) { best = p; bestDate = d; }
      }
      return Object.assign({ name: C.getCity(x.code).name, price: best, date: bestDate, dateText: D.md(bestDate) }, x);
    });
    this.setData({ deals });
  },
  setTrip(e) {
    this.setData({ trip: e.currentTarget.dataset.t });
    this.saveLast();
  },
  // 城市互换：图标转半圈，两边城市名淡出 → 换 → 淡入
  swap() {
    this.animate('#swap', [{ rotate: 0 }, { rotate: 180 }], 300, () => this.clearAnimation('#swap'));
    const kf = [{ opacity: 1, translateY: 0 }, { opacity: 0, translateY: -12, offset: 0.5 }, { opacity: 1, translateY: 0 }];
    this.animate('#from-name', kf, 300, () => this.clearAnimation('#from-name'));
    this.animate('#to-name', kf, 300, () => this.clearAnimation('#to-name'));
    setTimeout(() => {
      this.setData({ from: this.data.to, to: this.data.from });
      this.saveLast();
      this.loadDeals();
    }, 150);
  },
  pickCity(e) {
    const type = e.currentTarget.dataset.type;
    wx.navigateTo({
      url: '/pages/city/city?type=' + type,
      events: {
        pickCity: (res) => {
          const c = this.pick(C.getCity(res.code));
          const other = res.type === 'from' ? this.data.to : this.data.from;
          if (other && other.code === c.code) {
            // 选成同一城市：两边互换，和主流 App 一致
            this.setData({ from: this.data.to, to: this.data.from });
          } else {
            this.setData(res.type === 'from' ? { from: c } : { to: c });
          }
          this.saveLast();
          this.loadDeals();
        }
      },
      success: (r) => r.eventChannel.emit('init', { type, current: this.data[type].code })
    });
  },
  pickDate() {
    const d = this.data;
    const mode = d.trip === 'round' ? 'range' : 'single';
    wx.navigateTo({
      url: '/pages/calendar/calendar?mode=' + mode + '&from=' + d.from.code + '&to=' + d.to.code + '&dep=' + d.dep + '&ret=' + d.ret,
      events: {
        pickDate: (res) => this.setDates(res.dep, res.ret || (D.diffDays(res.dep, this.data.ret) > 0 ? this.data.ret : D.addDays(res.dep, 3)))
      },
      success: (r) => r.eventChannel.emit('init', { mode, dep: d.dep, ret: d.ret })
    });
  },
  pickCabin() {
    wx.showActionSheet({
      itemList: CABINS,
      success: (res) => { this.setData({ cabin: res.tapIndex, cabinName: CABINS[res.tapIndex] }); this.saveLast(); }
    });
  },
  search() {
    const d = this.data;
    if (d.from.code === d.to.code) return wx.showToast({ title: '出发与到达城市不能相同', icon: 'none' });
    if (d.trip === 'round' && D.diffDays(d.dep, d.ret) < 0) return wx.showToast({ title: '返程日期不能早于去程', icon: 'none' });
    store.addHistory({ from: d.from.code, to: d.to.code, fromName: d.from.name, toName: d.to.name, trip: d.trip });
    app.globalData.booking = { trip: d.trip, from: d.from.code, to: d.to.code, dep: d.dep, ret: d.ret, cabin: d.cabin, legs: [] };
    wx.navigateTo({ url: '/pages/list/list?from=' + d.from.code + '&to=' + d.to.code + '&date=' + d.dep + '&cabin=' + d.cabin + '&leg=1' });
  },
  useHistory(e) {
    const h = this.data.history[e.currentTarget.dataset.i];
    this.setData({ from: this.pick(C.getCity(h.from)), to: this.pick(C.getCity(h.to)), trip: h.trip });
    this.saveLast();
    this.loadDeals();
  },
  clearHistory() {
    wx.showModal({
      title: '清除搜索历史', content: '确定清除全部搜索记录吗？',
      success: (r) => { if (r.confirm) { store.clearHistory(); this.setData({ history: [] }); } }
    });
  },
  goDeal(e) {
    const x = this.data.deals[e.currentTarget.dataset.i];
    this.setData({ to: this.pick(C.getCity(x.code)), trip: 'one' });
    this.setDates(x.date, D.addDays(x.date, 3));
    this.search();
  },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  goPax() { wx.navigateTo({ url: '/pages/passengers/passengers' }); },
  showRules() {
    wx.showModal({
      title: '退改签说明',
      content: '退改费用按航司公布的规则、以申请时距起飞的时间计算：起飞前 7 天以上最低，起飞前 4 小时内最高。特价舱不得签转。具体以订单里的规则为准。',
      showCancel: false
    });
  },
  callService() {
    wx.makePhoneCall({
      phoneNumber: '4008207788',
      fail: () => wx.setClipboardData({ data: '400-820-7788', success: () => wx.showToast({ title: '客服电话已复制', icon: 'none' }) })
    });
  }
});
