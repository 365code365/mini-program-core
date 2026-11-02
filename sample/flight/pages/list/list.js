const D = require('../../utils/date');
const C = require('../../utils/cities');
const F = require('../../utils/flights');
const trend = require('./trend');
const app = getApp();

const SORTS = ['推荐排序', '价格从低到高', '起飞时间从早到晚', '飞行时长从短到长'];
const SLOTS = [{ k: 0, t: '00:00-06:00', a: 0, b: 360 }, { k: 1, t: '06:00-12:00', a: 360, b: 720 },
  { k: 2, t: '12:00-18:00', a: 720, b: 1080 }, { k: 3, t: '18:00-24:00', a: 1080, b: 1440 }];
const EMPTY_FILTER = { airlines: [], slots: [], airports: [], direct: false };

Page({
  data: {
    from: '', to: '', date: '', cabin: 0, leg: 1, loading: true, list: [], total: 0,
    days: [], dayView: '', sort: 0, sortName: SORTS[0], showFilter: false, filter: EMPTY_FILTER, draft: EMPTY_FILTER,
    filterCount: 0, airlineOpts: [], slotOpts: SLOTS, airportOpts: [], firstLeg: null, trendTip: ''
  },
  onLoad(q) {
    const from = q.from || 'HGH', to = q.to || 'BJS';
    this.setData({ from, to, date: q.date || D.addDays(D.today(), 1), cabin: +(q.cabin || 0), leg: +(q.leg || 1) });
    const b = app.globalData.booking;
    if (this.data.leg === 2 && b && b.legs[0]) {
      const l = b.legs[0];
      this.setData({ firstLeg: { no: l.flight.no, text: D.md(l.flight.date) + ' ' + l.flight.depText + ' ' + l.flight.fromName + '→' + l.flight.toName, price: l.cabin.price } });
    }
    wx.setNavigationBarTitle({ title: C.getCity(from).name + ' → ' + C.getCity(to).name + (this.data.leg === 2 ? '（返程）' : '') });
    this.setData({ airportOpts: C.getCity(from).airports.length > 1 ? C.getCity(from).airports.map(a => ({ code: a.code, name: a.name })) : [] });
    this.load();
  },
  onReady() { this.drawTrend(); },
  load() {
    const { from, to, date } = this.data;
    const start = D.diffDays(D.today(), date) > 3 ? D.addDays(date, -3) : D.today();
    const days = [];
    for (let i = 0; i < 15; i++) {
      const d = D.addDays(start, i);
      days.push({ date: d, md: D.mdShort(d), wk: D.weekLabel(d), price: F.lowPrice(from, to, d) });
    }
    this.setData({ days, dayView: 'd' + date, loading: true });
    clearTimeout(this._t);
    // 模拟实时查询的等待（真实场景是请求航司运价），期间显示加载态
    this._t = setTimeout(() => {
      this.all = F.genFlights(from, to, date);
      const al = {};
      this.all.forEach(f => { al[f.airline] = f.airlineName; });
      this.setData({ loading: false, airlineOpts: Object.keys(al).map(k => ({ code: k, name: al[k] })) });
      this.apply();
    }, 350);
  },
  apply() {
    const { filter, sort, cabin } = this.data;
    let list = (this.all || []).filter(f =>
      (!filter.airlines.length || filter.airlines.indexOf(f.airline) >= 0) &&
      (!filter.slots.length || filter.slots.some(k => f.dep >= SLOTS[k].a && f.dep < SLOTS[k].b)) &&
      (!filter.airports.length || filter.airports.indexOf(f.depCode) >= 0) &&
      (!filter.direct || !f.stop));
    list = list.map(f => {
      const c = cabin === 1 ? f.biz[0] : f.econ[0];
      return { no: f.no, airline: f.airline, airlineName: f.airlineName, logo: f.logo, depText: f.depText, arrText: f.arrText,
        depAirport: f.depAirport.replace(C.getCity(f.fromCode).name, '') + f.depTerm, arrAirport: f.arrAirport.replace(C.getCity(f.toCode).name, '') + f.arrTerm,
        durText: f.durText, cross: f.cross, stop: f.stop, aircraft: f.aircraft, meal: f.meal, onTime: f.onTime,
        price: c.price, discount: c.discount, cabinName: cabin === 1 ? '公务舱' : '经济舱', seats: c.seats, dep: f.dep, duration: f.duration };
    });
    if (sort === 1) list.sort((a, b) => a.price - b.price);
    if (sort === 2) list.sort((a, b) => a.dep - b.dep);
    if (sort === 3) list.sort((a, b) => a.duration - b.duration);
    if (sort === 0) list.sort((a, b) => (a.price + a.duration * 2 + (a.stop ? 300 : 0)) - (b.price + b.duration * 2 + (b.stop ? 300 : 0)));
    const n = filter.airlines.length + filter.slots.length + filter.airports.length + (filter.direct ? 1 : 0);
    this.setData({ list, total: (this.all || []).length, filterCount: n });
  },
  pickDay(e) {
    const date = e.currentTarget.dataset.d;
    if (date === this.data.date) return;
    this.setData({ date });
    this.load();
    this.drawTrend();
  },
  drawTrend() {
    trend.draw(this, '#trend', this.data.from, this.data.to, this.data.date, (tip) => this.setData({ trendTip: tip }));
  },
  tapTrend(e) {
    const d = trend.hit(this, e);
    if (d) this.pickDay({ currentTarget: { dataset: { d } } });
  },
  chooseSort() {
    wx.showActionSheet({
      itemList: SORTS,
      success: (r) => { this.setData({ sort: r.tapIndex, sortName: SORTS[r.tapIndex] }); this.apply(); }
    });
  },
  openFilter() { this.setDraft(JSON.parse(JSON.stringify(this.data.filter))); this.setData({ showFilter: true }); },
  // 模板里不做 indexOf（引擎的模板表达式对方法调用不可靠），选中态在这里算好
  setDraft(draft) {
    const on = (k, v) => draft[k].indexOf(v) >= 0;
    this.setData({
      draft,
      airlineOpts: this.data.airlineOpts.map(o => Object.assign({}, o, { on: on('airlines', o.code) })),
      slotOpts: SLOTS.map(o => Object.assign({}, o, { on: on('slots', o.k) })),
      airportOpts: this.data.airportOpts.map(o => Object.assign({}, o, { on: on('airports', o.code) }))
    });
  },
  closeFilter() { this.setData({ showFilter: false }); },
  // 引擎问题绕开：page-container 面板里的点击会同时派一次遮罩的 clickoverlay（面板与遮罩是兄弟节点，本不该冒泡过去）。
  // 面板根节点先记下点击时刻，紧跟着到的 clickoverlay 当成同一下点击，忽略
  markPanel() { this._panelTap = Date.now(); },
  onOverlay() { if (Date.now() - (this._panelTap || 0) > 300) this.closeFilter(); },
  toggle(e) {
    const { k, v } = e.currentTarget.dataset;
    const draft = JSON.parse(JSON.stringify(this.data.draft));
    const i = draft[k].indexOf(v);
    if (i >= 0) draft[k].splice(i, 1); else draft[k].push(v);
    this.setDraft(draft);
  },
  toggleDirect(e) { this.setData({ 'draft.direct': e.detail.value }); },
  resetFilter() { this.setDraft(JSON.parse(JSON.stringify(EMPTY_FILTER))); },
  confirmFilter() {
    this.setData({ filter: this.data.draft, showFilter: false });
    this.apply();
  },
  clearFilter() { this.setData({ filter: JSON.parse(JSON.stringify(EMPTY_FILTER)) }); this.apply(); },
  open(e) {
    const { from, to, date, cabin, leg } = this.data;
    wx.navigateTo({ url: '/pages/flight/flight?from=' + from + '&to=' + to + '&date=' + date + '&no=' + e.currentTarget.dataset.no + '&cabin=' + cabin + '&leg=' + leg });
  },
  onUnload() { clearTimeout(this._t); }
});
