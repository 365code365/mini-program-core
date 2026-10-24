// 行程的本地存储与日期工具。日期统一用 'YYYY-MM-DD' 字符串，计算时手动拆分，不依赖 Date 的字符串解析。
const { spotsOf, PACKING } = require('./data.js');

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
    budget: { total: '', items: {} }, packing: { checked: {}, custom: {} }, createdAt: Date.now(), pinned: false
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

module.exports = { pad, toStr, parse, today, addDays, diff, short, md, week, money, newTrip, all, save, get, put, remove, budgetSum, packingStat };
