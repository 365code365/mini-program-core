// 行程的本地存储与日期工具。日期统一用 'YYYY-MM-DD' 字符串，计算时手动拆分，不依赖 Date 的字符串解析。
const { spotsOf, PACKING, BUDGET_ITEMS } = require('./data.js');

const KEY = 'tv_trips';
const DAY = 86400000;

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function toStr(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function parse(s) { const p = s.split('-').map(Number); return new Date(p[0], p[1] - 1, p[2]); }
function today() { const d = new Date(); return new Date(d.getFullYear(), d.getMonth(), d.getDate()); }
function addDays(s, n) { return toStr(new Date(parse(s).getTime() + n * DAY)); }
function diff(a, b) { return Math.round((parse(b).getTime() - parse(a).getTime()) / DAY); }
function short(s) { const d = parse(s); return (d.getMonth() + 1) + '月' + d.getDate() + '日'; }
function md(s) { const d = parse(s); return pad(d.getMonth() + 1) + '/' + pad(d.getDate()); }
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
function week(s) { return WEEK[parse(s).getDay()]; }

function money(v) {
  const n = Math.round(Number(v || 0) * 100) / 100;
  return n % 1 === 0 ? String(n) : n.toFixed(2);
}

function newTrip(info) {
  const days = diff(info.start, info.end) + 1;
  const plan = [];
  for (let i = 0; i < days; i++) plan.push([]);
  return Object.assign({
    id: 't' + Date.now(), title: info.city + ' ' + days + ' 日游', type: '休闲度假', people: 1, plan,
    budget: { total: '', items: {} }, expenses: [], packing: { checked: {}, custom: {} }, createdAt: Date.now(), pinned: false
  }, info);
}

function seed() {
  const t0 = toStr(today());
  const a = newTrip({ city: '成都', start: addDays(t0, 12), end: addDays(t0, 15), people: 2, type: '美食之旅' });
  a.id = 'seed1';
  const s = spotsOf('成都');
  a.plan = [[s[0], s[1]], [s[3], s[4], s[5]], [s[7]], [s[6], s[9]]];
  a.budget = { total: '6000', items: { traffic: '1860', hotel: '1680', food: '1200', ticket: '370' } };
  a.packing = { checked: { '证件|身份证': true, '电子|充电宝': true, '衣物|外套': true }, custom: {} };
  a.expenses = [
    { id: 'e1', k: 'traffic', amount: '1860', note: '往返机票 × 2', day: 0, at: t0 },
    { id: 'e2', k: 'hotel', amount: '1680', note: '春熙路酒店 3 晚', day: 0, at: t0 }
  ];
  const b = newTrip({ city: '大理', start: addDays(t0, 40), end: addDays(t0, 44), people: 3, type: '休闲度假' });
  b.id = 'seed2';
  const d = spotsOf('大理');
  b.plan = [[d[1]], [d[0], d[4]], [d[3], d[2]], [d[5]], []];
  return [a, b];
}

function all() {
  let list = wx.getStorageSync(KEY);
  if (!list) { list = seed(); wx.setStorageSync(KEY, list); }
  return list;
}
function save(list) { wx.setStorageSync(KEY, list); }
function get(id) { return all().find(t => t.id === id); }
function put(trip) {
  const list = all();
  const i = list.findIndex(t => t.id === trip.id);
  if (i >= 0) list[i] = trip; else list.unshift(trip);
  save(list);
  return trip;
}
function remove(id) { save(all().filter(t => t.id !== id)); }

function budgetSum(b) {
  let s = 0;
  Object.keys(b.items || {}).forEach(k => { s += Number(b.items[k] || 0); });
  return Math.round(s * 100) / 100;
}
function packingStat(p) {
  let total = 0, done = 0;
  PACKING.forEach(g => {
    const items = g.items.concat((p.custom || {})[g.g] || []);
    items.forEach(it => { total++; if (p.checked[g.g + '|' + it]) done++; });
  });
  return { total, done, pct: total ? Math.round(done / total * 100) : 0 };
}


// ───────── 记账：实际支出（预算是计划，记账是真花了多少）─────────
function expenses(t) { return t.expenses || []; }
function expenseSum(t) {
  let s = 0;
  expenses(t).forEach(e => { s += Number(e.amount || 0); });
  return Math.round(s * 100) / 100;
}
function expenseByCat(t) {
  const m = {};
  expenses(t).forEach(e => { m[e.k] = Math.round(((m[e.k] || 0) + Number(e.amount || 0)) * 100) / 100; });
  return m;
}
function addExpense(id, e) {
  const t = get(id);
  t.expenses = [Object.assign({ id: 'e' + Date.now() }, e)].concat(expenses(t));
  put(t);
  return t;
}
function removeExpense(id, eid) {
  const t = get(id);
  t.expenses = expenses(t).filter(e => e.id !== eid);
  put(t);
  return t;
}

// ───────── 编辑：改日期后天数变了 ─────────
// 天数变多补空天；变少时被砍掉的那几天的景点并到最后一天（不丢数据），返回并过去的景点数
function resizePlan(t, start, end) {
  const days = diff(start, end) + 1;
  const plan = t.plan.slice(0, days).map(d => d.slice());
  while (plan.length < days) plan.push([]);
  let moved = 0;
  t.plan.slice(days).forEach(d => { d.forEach(s => { plan[days - 1].push(s); moved++; }); });
  t.plan = plan;
  t.start = start;
  t.end = end;
  return moved;
}
// 被砍掉的天里一共有几个景点（编辑页提交前提示用）
function droppedSpots(t, start, end) {
  const days = diff(start, end) + 1;
  return t.plan.slice(days).reduce((n, d) => n + d.length, 0);
}

// ───────── 复制 / 分享 ─────────
// 复制一份：景点、预算计划、自定义行李都带上；勾选状态与记账是「这一次」的，不带
function copyTrip(id) {
  const t = get(id);
  const c = JSON.parse(JSON.stringify(t));
  c.id = 't' + Date.now();
  c.title = (t.title + '（副本）').slice(0, 20);
  c.pinned = false;
  c.createdAt = Date.now();
  c.expenses = [];
  c.packing = { checked: {}, custom: (t.packing && t.packing.custom) || {} };
  return put(c);
}
const SHARE_HEAD = '【途简行程】';
// 分享文字：人能读，也能被「导入行程」原样解析回来
function shareText(t) {
  const lines = [SHARE_HEAD + t.title];
  lines.push(t.city + ' · ' + t.start + ' 至 ' + t.end + ' · ' + t.people + ' 人 · ' + t.type);
  t.plan.forEach((d, i) => {
    const date = addDays(t.start, i);
    lines.push('第' + (i + 1) + '天 ' + md(date) + ' ' + week(date) + '：' + (d.length ? d.map(s => s.name.replace(/[、：]/g, ' ')).join('、') : '自由活动'));
  });
  const total = Number(t.budget.total || 0);
  const parts = BUDGET_ITEMS.filter(b => Number((t.budget.items || {})[b.k] || 0) > 0).map(b => b.t + ' ' + money(t.budget.items[b.k]));
  if (total || parts.length) lines.push('预算' + (total ? ' ¥' + money(total) : '') + (parts.length ? '（' + parts.join(' / ') + '）' : ''));
  lines.push('复制整段文字，打开「途简行程」→ 我的 → 导入行程');
  return lines.join('\n');
}
// 解析分享文字；不是分享文字返回 null
function parseShare(text) {
  const lines = String(text || '').split('\n').map(l => l.trim()).filter(l => l);
  const h = lines.findIndex(l => l.indexOf(SHARE_HEAD) === 0);
  if (h < 0 || !lines[h + 1]) return null;
  const title = lines[h].slice(SHARE_HEAD.length).trim();
  const m = lines[h + 1].match(/^(.+?) · (\d{4}-\d{2}-\d{2}) 至 (\d{4}-\d{2}-\d{2}) · (\d+) 人 · (.+)$/);
  if (!m || diff(m[2], m[3]) < 0) return null;
  const city = m[1];
  const known = spotsOf(city);
  const plan = [];
  lines.slice(h + 2).forEach(l => {
    const d = l.match(/^第(\d+)天[^：]*：(.*)$/);
    if (!d) return;
    const names = d[2] === '自由活动' ? [] : d[2].split('、').map(x => x.trim()).filter(x => x);
    plan[Number(d[1]) - 1] = names.map(n => known.find(s => s.name === n) || { name: n, type: '休闲', hours: 1.5, ticket: '待定' });
  });
  const days = diff(m[2], m[3]) + 1;
  for (let i = 0; i < days; i++) plan[i] = plan[i] || [];
  plan.length = days;
  const budget = { total: '', items: {} };
  const b = lines.find(l => l.indexOf('预算') === 0);
  if (b) {
    const tm = b.match(/^预算 ¥([\d.]+)/);
    if (tm) budget.total = tm[1];
    BUDGET_ITEMS.forEach(it => { const im = b.match(new RegExp(it.t + ' ([\\d.]+)')); if (im) budget.items[it.k] = im[1]; });
  }
  return { title: title || city + ' ' + days + ' 日游', city, start: m[2], end: m[3], people: Number(m[4]), type: m[5], plan, budget };
}
function importTrip(info) {
  const t = newTrip({ city: info.city, start: info.start, end: info.end, people: info.people, type: info.type, title: info.title });
  t.plan = info.plan;
  t.budget = info.budget;
  return put(t);
}
module.exports = {
  pad, toStr, parse, today, addDays, diff, short, md, week, money, newTrip, all, save, get, put, remove, budgetSum, packingStat,
  expenses, expenseSum, expenseByCat, addExpense, removeExpense, resizePlan, droppedSpots, copyTrip, shareText, parseShare, importTrip
};
