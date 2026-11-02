const T = require('../../utils/store.js');
const { BUDGET_ITEMS } = require('../../utils/data.js');

// 受控金额输入：只留数字和一个小数点，小数最多两位，整数部分最多 6 位
function cleanMoney(v) {
  let s = String(v || '').replace(/[^\d.]/g, '');
  const dot = s.indexOf('.');
  if (dot >= 0) s = s.slice(0, dot + 1) + s.slice(dot + 1).replace(/\./g, '').slice(0, 2);
  let int = dot >= 0 ? s.slice(0, dot) : s;
  const dec = dot >= 0 ? s.slice(dot) : '';
  int = int.replace(/^0+(?=\d)/, '').slice(0, 6);
  if (dot === 0) int = '0';
  return int + dec;
}

Page({
  data: { rec: null, records: [], actual: [], spentText: '0', spentNote: '', spentOver: false, dayNames: [], total: '', items: [], sumText: '0', leftText: '0', pct: 0, over: false, overText: '', perText: '0', people: 1, segs: [] },
  onLoad(q) {
    this.id = q.id || (T.all()[0] || {}).id;
    if (!this.id) return;
    this.trip = T.get(this.id);
    this.vals = Object.assign({}, this.trip.budget.items);
    const dayNames = this.trip.plan.map((d, i) => '第' + (i + 1) + '天 ' + T.md(T.addDays(this.trip.start, i)));
    this.setData({ total: this.trip.budget.total || '', people: this.trip.people, dayNames });
    this.calc();
  },
  onUnload() { this.persist(); },
  calc() {
    const total = Number(this.data.total || 0);
    let sum = 0;
    BUDGET_ITEMS.forEach(b => { sum += Number(this.vals[b.k] || 0); });
    sum = Math.round(sum * 100) / 100;
    const base = Math.max(total, sum) || 1;
    const items = BUDGET_ITEMS.map(b => {
      const v = Number(this.vals[b.k] || 0);
      return { k: b.k, t: b.t, color: b.color, val: this.vals[b.k] || '', w: Math.round(v / base * 100), share: sum ? Math.round(v / sum * 100) : 0 };
    });
    const over = total > 0 && sum > total;
    this.setData({
      items, sumText: T.money(sum), pct: total ? Math.min(100, Math.round(sum / total * 100)) : 0, over,
      leftText: T.money(Math.max(0, total - sum)), overText: over ? T.money(sum - total) : '',
      perText: T.money(sum / this.data.people),
      segs: items.filter(i => i.w > 0).map(i => ({ k: i.k, color: i.color, w: i.w }))
    });
    this.calcSpent();
  },
  // —— 记账：实际花费与计划对比 ——
  calcSpent() {
    const t = T.get(this.id);
    const by = T.expenseByCat(t);
    const spent = T.expenseSum(t);
    const total = Number(this.data.total || 0);
    const actual = BUDGET_ITEMS.map(b => {
      const plan = Number(this.vals[b.k] || 0);
      const v = by[b.k] || 0;
      return { k: b.k, t: b.t, color: b.color, spent: T.money(v), plan: plan ? T.money(plan) : '', over: plan > 0 && v > plan };
    });
    const names = this.data.dayNames || [];
    const records = T.expenses(t).map(e => {
      const b = BUDGET_ITEMS.find(x => x.k === e.k) || BUDGET_ITEMS[5];
      return { id: e.id, t: b.t, color: b.color, note: e.note, amountText: T.money(e.amount), dayText: (names[e.day] || '').trim() };
    });
    let spentNote = '还没设总预算';
    if (total) spentNote = spent > total ? '超出总预算 ¥' + T.money(spent - total) : '总预算还剩 ¥' + T.money(total - spent);
    this.setData({ actual, records, spentText: T.money(spent), spentOver: total > 0 && spent > total, spentNote });
  },
  openRec() {
    // 默认记到「今天」所在的那一天（还没出发 / 已结束就记第 1 天）
    const d = T.diff(this.trip.start, T.toStr(T.today()));
    const day = d >= 0 && d < this.trip.plan.length ? d : 0;
    this.setData({ rec: { k: 'food', amount: '', note: '', day } });
  },
  closeRec() { this.setData({ rec: null }); },
  noop() {},
  onRecCat(e) { this.setData({ 'rec.k': e.currentTarget.dataset.k }); },
  onRecAmount(e) { const v = cleanMoney(e.detail.value); this.setData({ 'rec.amount': v }); return v; },
  onRecNote(e) { this.setData({ 'rec.note': e.detail.value.slice(0, 20) }); },
  onRecDay(e) { this.setData({ 'rec.day': Number(e.detail.value) }); },
  saveRec() {
    const r = this.data.rec;
    if (!(Number(r.amount) > 0)) { wx.showToast({ title: '请输入金额', icon: 'none' }); return; }
    T.addExpense(this.id, { k: r.k, amount: r.amount, note: r.note.trim(), day: r.day, at: T.toStr(T.today()) });
    this.setData({ rec: null });
    this.calcSpent();
    wx.showToast({ title: '已记账 ¥' + T.money(r.amount), icon: 'none' });
  },
  delRec(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '删除这笔记账', content: '删除后实际花费会相应减少', confirmText: '删除', confirmColor: '#f5222d',
      success: r => { if (r.confirm) { T.removeExpense(this.id, id); this.calcSpent(); } }
    });
  },
  onTotal(e) {
    const v = cleanMoney(e.detail.value);
    this.setData({ total: v });
    this.calc();
    this.persistLater();
    return v;
  },
  onItem(e) {
    const k = e.currentTarget.dataset.k;
    const v = cleanMoney(e.detail.value);
    this.vals[k] = v;
    this.calc();
    this.persistLater();
    return v;
  },
  clearItem(e) {
    this.vals[e.currentTarget.dataset.k] = '';
    this.calc();
    this.persistLater();
  },
  quick(e) {
    this.setData({ total: e.currentTarget.dataset.v });
    this.calc();
    this.persistLater();
  },
  persistLater() {
    clearTimeout(this.pt);
    this.pt = setTimeout(() => this.persist(), 500);
  },
  // 写回 storage；详情页 onShow 时从 storage 重新读，就拿到了新的预算
  persist() {
    clearTimeout(this.pt);
    const t = T.get(this.id);
    if (!t) return;
    t.budget = { total: this.data.total, items: Object.assign({}, this.vals) };
    T.put(t);
  },
  done() { this.persist(); wx.navigateBack(); }
});
