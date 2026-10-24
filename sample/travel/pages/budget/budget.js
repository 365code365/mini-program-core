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
  data: { total: '', items: [], sumText: '0', leftText: '0', pct: 0, over: false, overText: '', perText: '0', people: 1, segs: [] },
  onLoad(q) {
    this.id = q.id || (T.all()[0] || {}).id;
    if (!this.id) return;
    this.trip = T.get(this.id);
    this.vals = Object.assign({}, this.trip.budget.items);
    this.setData({ total: this.trip.budget.total || '', people: this.trip.people });
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
