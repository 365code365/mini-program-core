const D = require('../../utils/date');
const F = require('../../utils/flights');
const app = getApp();

// 航班详情 + 舱位：经济舱三档运价 / 公务舱头等舱，每档看退改规则，选一档进入下一步（往返先选返程）
Page({
  data: { f: null, tab: 0, fares: [], rules: null, showRules: false, tax: 0, dateText: '', leg: 1, trip: 'one' },
  onLoad(q) {
    const date = q.date || D.addDays(D.today(), 1);
    let f = F.findFlight(q.from || 'HGH', q.to || 'BJS', date, q.no);
    if (!f) f = F.genFlights(q.from || 'HGH', q.to || 'BJS', date)[0];
    this.f = f;
    const t = F.taxes(f.dist);
    const b = app.globalData.booking;
    this.setData({
      f: { no: f.no, airlineName: f.airlineName, logo: f.logo, depText: f.depText, arrText: f.arrText, depAirport: f.depAirport + f.depTerm,
        arrAirport: f.arrAirport + f.arrTerm, durText: f.durText, cross: f.cross, stop: f.stop, aircraft: f.aircraft, meal: f.meal,
        onTime: f.onTime, fromName: f.fromName, toName: f.toName, dist: f.dist },
      dateText: D.md(date) + ' ' + D.weekLabel(date), tab: +(q.cabin || 0), tax: t.fund + t.fuel,
      leg: +(q.leg || 1), trip: b ? b.trip : 'one'
    });
    wx.setNavigationBarTitle({ title: f.fromName + ' → ' + f.toName });
    this.showFares();
  },
  showFares() {
    const src = this.data.tab === 1 ? this.f.biz : this.f.econ;
    const fares = src.map(c => {
      const rows = F.ruleRows(c.rule, c.price);
      return Object.assign({}, c, { refundFrom: rows[0].refund, changeFrom: rows[0].change, endorse: F.RULES[c.rule].endorse });
    });
    this.setData({ fares });
  },
  setTab(e) { this.setData({ tab: +e.currentTarget.dataset.t }); this.showFares(); },
  openRules(e) {
    const c = this.data.fares[e.currentTarget.dataset.i];
    this.setData({ showRules: true, rules: { name: c.name, price: c.price, endorse: c.endorse, bag: c.bag, rows: F.ruleRows(c.rule, c.price) } });
  },
  closeRules() { this.setData({ showRules: false }); },
  noop() {},
  book(e) {
    const c = this.data.fares[e.currentTarget.dataset.i];
    const f = this.f;
    let b = app.globalData.booking;
    if (!b) b = app.globalData.booking = { trip: 'one', from: f.fromCode, to: f.toCode, dep: f.date, ret: '', cabin: this.data.tab, legs: [] };
    const leg = this.data.leg;
    b.legs = b.legs.slice(0, leg - 1);
    b.legs.push({
      flight: { no: f.no, airline: f.airline, airlineName: f.airlineName, logo: f.logo, date: f.date, fromCode: f.fromCode, toCode: f.toCode,
        fromName: f.fromName, toName: f.toName, depText: f.depText, arrText: f.arrText, dep: f.dep, cross: f.cross, durText: f.durText,
        depAirport: f.depAirport + f.depTerm, arrAirport: f.arrAirport + f.arrTerm, aircraft: f.aircraft, stop: f.stop, dist: f.dist, full: f.full },
      cabin: { id: c.id, name: c.name, price: c.price, rule: c.rule, bag: c.bag, discount: c.discount }
    });
    if (b.trip === 'round' && leg === 1) {
      wx.showToast({ title: '去程已选，请选返程', icon: 'none' });
      wx.navigateTo({ url: '/pages/list/list?from=' + f.toCode + '&to=' + f.fromCode + '&date=' + b.ret + '&cabin=' + this.data.tab + '&leg=2' });
      return;
    }
    wx.navigateTo({ url: '/pages/order-fill/order-fill' });
  }
});
