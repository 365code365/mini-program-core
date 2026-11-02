// 跨页数据：加油记录、油卡余额、优惠券、发票抬头、已开发票、油号偏好，全部落 storage。
const { stations, COUPONS } = require('./data.js');

const K = { rec: 'fuel_records', card: 'fuel_card', cp: 'fuel_coupons', titles: 'fuel_titles', oil: 'fuel_oil', inited: 'fuel_inited' };
const DAY = 86400e3;

function get(k, def) { const v = wx.getStorageSync(k); return v === '' || v === undefined || v === null ? def : v; }
function set(k, v) { wx.setStorageSync(k, v); }
function money(n) { return (Math.round(n * 100) / 100).toFixed(2); }
function pad(n) { return n < 10 ? '0' + n : '' + n; }
function fmt(ts, withYear) {
  const d = new Date(ts);
  const md = `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  return withYear ? `${d.getFullYear()}-${md}` : md;
}
function monthKey(ts) { const d = new Date(ts); return `${d.getFullYear()}-${pad(d.getMonth() + 1)}`; }

// 首次启动：一张新人券 + 近两个月的几笔历史记录（月度统计有东西可看）
function init() {
  if (get(K.inited, 0)) return;
  const now = Date.now();
  const hist = [[52, 's1', '95#', 3, 300], [38, 's7', '95#', 5, 260], [24, 's3', '95#', 4, 350], [9, 's1', '95#', 7, 280]];
  set(K.rec, hist.map(([ago, sid, oil, gun, amount], i) => {
    const st = stations.find((s) => s.id === sid);
    return makeRecord({ station: st, oil, gun, amount, couponCut: i === 2 ? 18 : 0, way: i % 2 ? 'card' : 'wx', at: now - ago * DAY - i * 3600e3, invoiced: i < 2 });
  }).reverse());
  set(K.cp, [grant(COUPONS[2], now)]);
  set(K.card, { balance: 0, no: '8800 2615 3307 4129', logs: [] });
  set(K.inited, 1);
}

// ---------- 记录 ----------
function makeRecord(o) {
  const pr = o.station.prices[o.oil];
  const liters = o.amount / pr.list;
  const discount = liters * (pr.list - pr.ours);
  const pay = Math.max(0.01, o.amount - discount - (o.couponCut || 0));
  const at = o.at || Date.now();
  return {
    id: 'r' + at, no: 'YJ' + fmt(at, true).replace(/[- :]/g, '') + String(at % 997).padStart(3, '0'),
    stationId: o.station.id, stationName: o.station.name, oil: o.oil, gun: o.gun, amount: money(o.amount), liters: liters.toFixed(2),
    list: pr.list, ours: pr.ours, discount: money(discount), couponCut: money(o.couponCut || 0), pay: money(pay), way: o.way || 'wx',
    createdAt: at, invoice: o.invoiced ? { title: '个人', type: 'person', at: at + 3600e3, email: '' } : null
  };
}
function records() { return get(K.rec, []); }
function addRecord(r) { const l = records(); l.unshift(r); set(K.rec, l); return r; }
function recordById(id) { return records().find((r) => r.id === id); }
function updateRecord(id, patch) {
  const l = records();
  const r = l.find((x) => x.id === id);
  if (r) Object.assign(r, patch);
  set(K.rec, l);
  return r;
}

// ---------- 油卡 ----------
function card() { return get(K.card, { balance: 0, no: '', logs: [] }); }
function cardChange(delta, text) {
  const c = card();
  c.balance = Math.round((c.balance + delta) * 100) / 100;
  c.logs.unshift({ at: Date.now(), delta: money(delta), text });
  c.logs = c.logs.slice(0, 30);
  set(K.card, c);
  return c;
}

// ---------- 优惠券 ----------
function grant(tpl, now) {
  return { id: 'u' + tpl.id + (now || Date.now()), tpl: tpl.id, title: tpl.title, min: tpl.min, cut: tpl.cut, expireAt: (now || Date.now()) + tpl.days * DAY, status: 'unused' };
}
function coupons() {
  const now = Date.now();
  return get(K.cp, []).map((c) => Object.assign({}, c, { status: c.status === 'unused' && c.expireAt < now ? 'expired' : c.status }));
}
function claim(tplId) {
  const tpl = COUPONS.find((c) => c.id === tplId);
  const list = get(K.cp, []);
  if (list.some((c) => c.tpl === tplId && c.status === 'unused' && c.expireAt > Date.now())) return null;
  const c = grant(tpl);
  list.unshift(c);
  set(K.cp, list);
  return c;
}
function useCoupon(id) {
  const list = get(K.cp, []);
  const c = list.find((x) => x.id === id);
  if (c) c.status = 'used';
  set(K.cp, list);
}
// 本单可用的券（未用、未过期、满足门槛），按减额从大到小
function usable(amount) {
  return coupons().filter((c) => c.status === 'unused' && amount >= c.min).sort((a, b) => b.cut - a.cut);
}

// ---------- 发票抬头 ----------
function titles() { return get(K.titles, []); }
function saveTitle(t) {
  const l = titles();
  if (t.isDefault) l.forEach((x) => { x.isDefault = false; });
  const i = l.findIndex((x) => x.id === t.id);
  if (i >= 0) l[i] = t; else l.unshift(Object.assign({ id: 't' + Date.now() }, t));
  if (l.length && !l.some((x) => x.isDefault)) l[0].isDefault = true;
  set(K.titles, l);
  return l;
}
function removeTitle(id) {
  const l = titles().filter((x) => x.id !== id);
  if (l.length && !l.some((x) => x.isDefault)) l[0].isDefault = true;
  set(K.titles, l);
  return l;
}

function oilPref() { return get(K.oil, '95#'); }
function setOilPref(o) { set(K.oil, o); }

module.exports = {
  money, pad, fmt, monthKey, init,
  makeRecord, records, addRecord, recordById, updateRecord,
  card, cardChange, coupons, claim, useCoupon, usable,
  titles, saveTitle, removeTitle, oilPref, setOilPref
};
