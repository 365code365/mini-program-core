const T = require('../../utils/store.js');
const { CITIES } = require('../../utils/data.js');

Page({
  data: { trip: null, dayTabs: [], day: 0, spots: [], dayHours: 0, budget: {}, pack: {}, undo: null, intoTab: '' },
  // 没带 id 时（直接打开这一页）展示第一个行程
  onLoad(q) { this.id = q.id || (T.all()[0] || {}).id; },
  onShow() { this.load(); },
  onUnload() { clearTimeout(this.undoTimer); },
  load() {
    const t = T.get(this.id);
    if (!t) { wx.navigateBack(); return; }
    this.trip = t;
    const city = CITIES.find(c => c.name === t.city) || { color: '#1e78ff' };
    const dayTabs = t.plan.map((d, i) => {
      const date = T.addDays(t.start, i);
      return { i, label: '第' + (i + 1) + '天', date: T.md(date) + ' ' + T.week(date), n: d.length };
    });
    const sum = T.budgetSum(t.budget);
    const total = Number(t.budget.total || 0);
    const pk = T.packingStat(t.packing);
    this.setData({
      trip: { title: t.title, city: t.city, color: city.color, range: T.short(t.start) + ' - ' + T.short(t.end), days: t.plan.length, people: t.people, type: t.type },
      dayTabs, day: Math.min(this.data.day, t.plan.length - 1),
      budget: { sumText: T.money(sum), totalText: total ? T.money(total) : '未设置', pct: total ? Math.min(100, Math.round(sum / total * 100)) : 0, over: total > 0 && sum > total },
      pack: pk
    });
    this.renderDay();
  },
  renderDay() {
    const list = this.trip.plan[this.data.day] || [];
    const last = list.length - 1;
    const spots = list.map((s, i) => Object.assign({}, s, { idx: i, no: i + 1, first: i === 0, last: i === last }));
    const hours = list.reduce((h, s) => h + Number(s.hours || 0), 0);
    this.setData({ spots, dayHours: Math.round(hours * 10) / 10 });
  },
  onDay(e) {
    const day = Number(e.currentTarget.dataset.i);
    this.setData({ day, intoTab: 'tab' + Math.max(0, day - 1) });
    this.renderDay();
  },
  save() {
    T.put(this.trip);
    const n = this.trip.plan[this.data.day].length;
    this.setData({ ['dayTabs[' + this.data.day + '].n']: n });
    this.renderDay();
  },
  up(e) {
    const i = Number(e.currentTarget.dataset.i);
    if (i <= 0) return;
    const l = this.trip.plan[this.data.day];
    const t = l[i - 1]; l[i - 1] = l[i]; l[i] = t;
    this.save();
  },
  down(e) {
    const i = Number(e.currentTarget.dataset.i);
    const l = this.trip.plan[this.data.day];
    if (i >= l.length - 1) return;
    const t = l[i + 1]; l[i + 1] = l[i]; l[i] = t;
    this.save();
  },
  // 删除后 4 秒内可撤销
  del(e) {
    const i = Number(e.currentTarget.dataset.i);
    const l = this.trip.plan[this.data.day];
    const removed = l.splice(i, 1)[0];
    this.save();
    clearTimeout(this.undoTimer);
    this.setData({ undo: { name: removed.name, day: this.data.day, i, spot: removed } });
    this.undoTimer = setTimeout(() => this.setData({ undo: null }), 4000);
  },
  undoDel() {
    const u = this.data.undo;
    if (!u) return;
    clearTimeout(this.undoTimer);
    this.trip.plan[u.day].splice(u.i, 0, u.spot);
    this.setData({ undo: null, day: u.day });
    this.save();
  },
  // 添加景点：EventChannel 接收景点页回传的选择结果
  addSpot() {
    const day = this.data.day;
    const taken = {};
    this.trip.plan.forEach((d, di) => d.forEach(s => { taken[s.name] = di + 1; }));
    wx.navigateTo({
      url: '/pages/spots/spots?city=' + encodeURIComponent(this.trip.city) + '&day=' + (day + 1),
      events: {
        picked: list => {
          const t = T.get(this.id);
          list.forEach(s => t.plan[day].push(s));
          T.put(t);
          wx.showToast({ title: '已添加 ' + list.length + ' 个景点', icon: 'none' });
        }
      },
      success: res => { res.eventChannel.emit('init', { taken }); }
    });
  },
  goBudget() { wx.navigateTo({ url: '/pages/budget/budget?id=' + this.id }); },
  goPacking() { wx.navigateTo({ url: '/pages/packing/packing?id=' + this.id }); }
});
