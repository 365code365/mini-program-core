// 本地数据：钱包、行程、骑行中状态、故障上报、设置。全部走 storage，退出重进还在
const geo = require('./geo.js');
const fee = require('./fee.js');

const K = { wallet: 'bike_wallet', trips: 'bike_trips', ride: 'bike_ride', faults: 'bike_faults', settings: 'bike_settings' };
const DAY = 86400000;

function get(key, def) {
  const v = wx.getStorageSync(key);
  return v === '' || v === undefined || v === null ? def : v;
}
function set(key, v) { wx.setStorageSync(key, v); }

// 第一次打开时的历史行程：最近一周在武林商圈的几次通勤
function seedTrips() {
  const now = Date.now();
  const plan = [
    [1, 8.6, 'bike', 0, 2, 960, 2.9, 'balance'],
    [1, 18.3, 'ebike', 3, 11, 1260, 6.1, 'wechat'],
    [2, 8.5, 'bike', 4, 0, 780, 2.3, 'balance'],
    [3, 12.2, 'bike', 2, 9, 1980, 5.4, 'balance'],
    [5, 19.1, 'ebike', 10, 5, 840, 4.0, 'wechat'],
    [6, 9.0, 'bike', 0, 3, 1140, 3.6, 'balance']
  ];
  return plan.map((p, i) => {
    const [daysAgo, hour, type, from, to, sec, km, payBy] = p;
    const d = new Date(now - daysAgo * DAY);
    d.setHours(Math.floor(hour), Math.round((hour % 1) * 60), 0, 0);
    const a = geo.POIS[from], b = geo.POIS[to];
    const f = fee.calc(type, sec);
    const no = (type === 'ebike' ? '8' : '1') + String(31270455 + i * 7731).padStart(8, '0');
    const end = d.getTime() + sec * 1000;
    return {
      id: 'T' + end, no, type, startAt: d.getTime(), endAt: end, sec, meters: Math.round(km * 1000),
      from: a.name, to: b.name, path: geo.track(a, b, i + 3),
      start: f.start, extra: f.extra, total: f.total, discount: 0, paid: f.total,
      status: 'paid', payBy, paidAt: end + 30000
    };
  }).sort((x, y) => y.startAt - x.startAt);
}

function seedWallet() {
  const now = Date.now();
  return {
    balance: 12.6,
    cards: [],
    txns: [
      { id: 'W1', title: '余额充值（赠 1 元）', amount: 21, at: now - 9 * DAY, kind: 'in' },
      { id: 'W2', title: '单车骑行 · 武林广场', amount: -1.5, at: now - 6 * DAY, kind: 'out' },
      { id: 'W3', title: '单车骑行 · 黄龙体育中心', amount: -3.5, at: now - 3 * DAY, kind: 'out' },
      { id: 'W4', title: '单车骑行 · 浙江展览馆', amount: -1.5, at: now - 2 * DAY, kind: 'out' },
      { id: 'W5', title: '单车骑行 · 西湖文化广场', amount: -1.9, at: now - DAY, kind: 'out' }
    ]
  };
}

function wallet() {
  let w = get(K.wallet, null);
  if (!w) { w = seedWallet(); set(K.wallet, w); }
  return w;
}
function saveWallet(w) { w.balance = fee.round2(w.balance); set(K.wallet, w); }
function addTxn(w, title, amount) {
  w.txns.unshift({ id: 'W' + Date.now(), title, amount: fee.round2(amount), at: Date.now(), kind: amount >= 0 ? 'in' : 'out' });
  if (w.txns.length > 50) w.txns.length = 50;
}

function trips() {
  let t = get(K.trips, null);
  if (!t) { t = seedTrips(); set(K.trips, t); }
  return t;
}
function saveTrips(t) { set(K.trips, t); }
function trip(id) { return trips().find(x => x.id === id) || null; }
function updateTrip(id, patch) {
  const t = trips();
  const i = t.findIndex(x => x.id === id);
  if (i >= 0) { t[i] = Object.assign({}, t[i], patch); saveTrips(t); return t[i]; }
  return null;
}
function unpaidTrip() { return trips().find(x => x.status === 'unpaid') || null; }

function ride() { return get(K.ride, null); }
function saveRide(r) { if (r) set(K.ride, r); else wx.removeStorageSync(K.ride); }

function faults() { return get(K.faults, []); }
function saveFaults(f) { set(K.faults, f); }
function isReported(no) { return faults().some(f => f.no === no && f.status !== 'done'); }

function settings() { return Object.assign({ vibrate: true, keepOn: true }, get(K.settings, {})); }
function saveSettings(s) { set(K.settings, s); }

function fmtTime(ts, withDate) {
  const d = new Date(ts);
  const p = n => String(n).padStart(2, '0');
  const t = p(d.getHours()) + ':' + p(d.getMinutes());
  return withDate === false ? t : (d.getMonth() + 1) + '月' + d.getDate() + '日 ' + t;
}

module.exports = {
  wallet, saveWallet, addTxn, trips, saveTrips, trip, updateTrip, unpaidTrip,
  ride, saveRide, faults, saveFaults, isReported, settings, saveSettings, fmtTime
};
