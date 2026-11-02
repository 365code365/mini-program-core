const T = require('../../utils/store.js');
const host = require('../../utils/host.js');
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
      budget: { sumText: T.money(sum), totalText: total ? T.money(total) : '未设置', pct: total ? Math.min(100, Math.round(sum / total * 100)) : 0, over: total > 0 && sum > total, spentText: T.money(T.expenseSum(t)) },
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
  // —— 移到其他天 ——
  openMove(e) {
    const i = Number(e.currentTarget.dataset.i);
    const s = this.trip.plan[this.data.day][i];
    if (this.trip.plan.length < 2) { wx.showToast({ title: '只有 1 天，没有别的天可移', icon: 'none' }); return; }
    this.setData({ move: { i, name: s.name } });
  },
  closeMove() { this.setData({ move: null }); },
  noop() {},
  moveTo(e) {
    const to = Number(e.currentTarget.dataset.i);
    const m = this.data.move;
    if (!m || to === this.data.day) return;
    const s = this.trip.plan[this.data.day].splice(m.i, 1)[0];
    this.trip.plan[to].push(s);
    T.put(this.trip);
    this.setData({ move: null, ['dayTabs[' + to + '].n']: this.trip.plan[to].length });
    this.save();
    wx.showToast({ title: '已移到第' + (to + 1) + '天', icon: 'none' });
  },
  // —— 行程本身：编辑 / 分享 / 复制 / 删除 ——
  edit() { wx.navigateTo({ url: '/pages/create/create?id=' + this.id }); },
  share() {
    wx.showActionSheet({
      itemList: ['复制行程文字（发给好友）', '查看分享内容'],
      success: r => {
        const text = T.shareText(T.get(this.id));
        if (r.tapIndex === 0) host.copy(text);
        else wx.showModal({ title: '分享内容', content: text, confirmText: '复制', success: m => { if (m.confirm) host.copy(text); } });
      }
    });
  },
  copy() {
    wx.showModal({
      title: '复制一份行程', content: '景点安排、预算计划和自定义行李会一起复制，勾选状态与记账不复制', confirmText: '复制',
      success: r => {
        if (!r.confirm) return;
        const c = T.copyTrip(this.id);
        wx.showToast({ title: '已复制为「' + c.title + '」', icon: 'none' });
        wx.redirectTo({ url: '/pages/detail/detail?id=' + c.id });
      }
    });
  },
  remove() {
    wx.showModal({
      title: '删除行程', content: '「' + this.trip.title + '」的景点、预算和行李清单会一起删除', confirmText: '删除', confirmColor: '#f5222d',
      success: r => {
        if (!r.confirm) return;
        T.remove(this.id);
        wx.showToast({ title: '已删除', icon: 'none' });
        wx.navigateBack();
      }
    });
  },
  // 右上角「转发」：好友打开的是导入页，分享文字里带着全部安排
  onShareAppMessage() {
    const t = T.get(this.id);
    return { title: t.title + ' · 我的行程安排', path: '/pages/import/import?text=' + encodeURIComponent(T.shareText(t)) };
  },
  goBudget() { wx.navigateTo({ url: '/pages/budget/budget?id=' + this.id }); },
  goPacking() { wx.navigateTo({ url: '/pages/packing/packing?id=' + this.id }); }
});
