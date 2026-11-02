// 本地状态：起终点 / 常用地址 / 行程 / 优惠券 / 钱包 / 紧急联系人 / 发票，全部存在 storage 里。
// 进行中的那一单（呼叫 → 接驾 → 行程中）的状态机在 order.js
const data = require('./data.js');
const K = {
  from: 'ride_from', to: 'ride_to', history: 'ride_history', trips: 'ride_trips',
  used: 'ride_coupon_used', extra: 'ride_coupon_extra', codes: 'ride_codes', claimed: 'ride_weekend_claimed',
  wallet: 'ride_wallet', places: 'ride_places', contacts: 'ride_contacts', invoices: 'ride_invoices',
  autoShare: 'ride_autoshare', record: 'ride_record', pick: 'ride_coupon_pick', type: 'ride_type', profile: 'ride_profile'
};
const round1 = data.round1;
function round2(n) {
  return Math.round(n * 100) / 100;
}
function pad(n) {
  return n < 10 ? '0' + n : '' + n;
}
function hm(ts) {
  const d = new Date(ts);
  return pad(d.getHours()) + ':' + pad(d.getMinutes());
}
// 「10月23日 08:12」：行程列表与详情的时间写法
function dayText(ts) {
  const d = new Date(ts);
  return (d.getMonth() + 1) + '月' + d.getDate() + '日 ' + hm(ts);
}
function dateTime(ts) {
  const d = new Date(ts);
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) + ' ' + hm(ts);
}
// storage 里没有这一项时 getStorageSync 给 ''：数值、开关都不能用 `|| 默认值`
function list(key) {
  const v = wx.getStorageSync(key);
  return Array.isArray(v) ? v : [];
}
function flag(key, def) {
  const v = wx.getStorageSync(key);
  return typeof v === 'boolean' ? v : def;
}
function maskPhone(p) {
  const s = String(p || '').replace(/\s/g, '');
  return s.length === 11 ? s.slice(0, 3) + '****' + s.slice(7) : s;
}

// ───────── 地点 ─────────
function place(p) {
  return p ? { name: p.name, addr: p.addr || '', x: p.x || 0, y: p.y || 0, airport: !!p.airport } : null;
}
// 旧版存的地点只有 { name, addr, km }、没有坐标：按名字回查 POI 补上，否则所有距离都会算成起步的 1.5 公里
function stored(v) {
  if (!v || !v.name) return null;
  if (typeof v.x === 'number' && typeof v.y === 'number') return v;
  return place(data.findPoi(v.name) || (v.name === data.ORIGIN.name ? data.ORIGIN : v));
}
function getFrom() {
  return stored(wx.getStorageSync(K.from)) || place(data.ORIGIN);
}
function setFrom(p) {
  wx.setStorageSync(K.from, place(p));
}
function getTo() {
  return stored(wx.getStorageSync(K.to));
}
function setTo(p) {
  if (p) wx.setStorageSync(K.to, place(p));
  else wx.removeStorageSync(K.to);
}
function history() {
  return list(K.history).map(stored).filter(Boolean);
}
function pushHistory(p) {
  const l = history().filter(h => h.name !== p.name);
  l.unshift(place(p));
  wx.setStorageSync(K.history, l.slice(0, 5));
}
function clearHistory() {
  wx.removeStorageSync(K.history);
}
// 常用地址：家 / 公司各一个，其他最多 5 个
function getPlaces() {
  const v = wx.getStorageSync(K.places);
  if (v && typeof v === 'object' && !Array.isArray(v)) return v;
  return { home: place(data.findPoi('望京西园三区')), company: place(data.findPoi('中关村软件园二期')), others: [] };
}
function setPlace(slot, p) {
  const v = getPlaces();
  if (slot === 'home' || slot === 'company') v[slot] = place(p);
  else {
    const others = (v.others || []).filter(o => o.name !== p.name);
    if (others.length >= 5) return false;
    others.push(place(p));
    v.others = others;
  }
  wx.setStorageSync(K.places, v);
  return true;
}
function removePlace(slot, i) {
  const v = getPlaces();
  if (slot === 'home' || slot === 'company') v[slot] = null;
  else v.others = (v.others || []).filter((o, k) => k !== i);
  wx.setStorageSync(K.places, v);
}
// 「我的」里那一行的小字：家 · 公司 / 未设置
function placesText() {
  const v = getPlaces();
  const l = [];
  if (v.home) l.push('家');
  if (v.company) l.push('公司');
  if ((v.others || []).length) l.push('其他 ' + v.others.length);
  return l.length ? l.join(' · ') : '未设置';
}
// 上一次选的车型：首页记住它；券页「去使用」限车型的券时切过去
function getType() {
  const v = wx.getStorageSync(K.type);
  return data.CAR_TYPES.some(c => c.key === v) ? v : 'express';
}
function setType(key) {
  wx.setStorageSync(K.type, data.carType(key).key);
}
function getProfile() {
  const v = wx.getStorageSync(K.profile);
  return Object.assign({ nick: '林晓舟', phone: '13852706621' }, v && typeof v === 'object' ? v : {});
}
function setProfile(patch) {
  wx.setStorageSync(K.profile, Object.assign(getProfile(), patch));
}

// ───────── 优惠券 ─────────
function allCoupons() {
  const used = list(K.used);
  return list(K.extra).concat(data.COUPONS).map(c => Object.assign({}, c, { used: used.indexOf(c.id) >= 0 }));
}
function validCoupons() {
  return allCoupons().filter(c => !c.used);
}
function isNight(ts) {
  const h = new Date(ts).getHours();
  return h >= 22 || h < 5;
}
// 这张券在这一单上能抵多少（不满足条件 0）。ctx：{ typeKey, from, to, amount, at }
function couponCut(c, ctx) {
  if (!c || c.used) return 0;
  if (c.types && c.types.indexOf(ctx.typeKey) < 0) return 0;
  if (c.airport && !/机场/.test((ctx.from ? ctx.from.name : '') + (ctx.to ? ctx.to.name : ''))) return 0;
  if (c.night && !isNight(ctx.at || Date.now())) return 0;
  if (ctx.amount < c.min) return 0;
  // 券不能把车费抵到 0：至少付 0.1 元
  return Math.max(0, round1(Math.min(c.amount, ctx.amount - 0.1)));
}
function usableCoupons(ctx) {
  return validCoupons().map(c => ({ coupon: c, cut: couponCut(c, ctx) })).filter(x => x.cut > 0).sort((a, b) => b.cut - a.cut);
}
function bestCoupon(ctx) {
  return usableCoupons(ctx)[0] || { coupon: null, cut: 0 };
}
// 券页点过「去使用」的那张：这一单能用就用它，用不了退回最省的
function getPick() {
  const v = wx.getStorageSync(K.pick);
  return typeof v === 'string' ? v : '';
}
function setPick(id) {
  if (id) wx.setStorageSync(K.pick, id);
  else wx.removeStorageSync(K.pick);
}
function pickFor(ctx) {
  const usable = usableCoupons(ctx);
  const want = getPick();
  return (want && usable.find(x => x.coupon.id === want)) || usable[0] || { coupon: null, cut: 0 };
}
function useCoupon(id) {
  const used = list(K.used);
  if (used.indexOf(id) < 0) used.push(id);
  wx.setStorageSync(K.used, used);
  if (getPick() === id) setPick('');
}
function grantCoupon(c, prefix) {
  const extra = list(K.extra);
  const end = new Date(Date.now() + 30 * 86400000);
  const coupon = Object.assign({}, c, {
    id: (prefix || 'g') + Date.now().toString(36) + extra.length,
    date: '领取后 30 天内有效（至 ' + end.getFullYear() + '.' + pad(end.getMonth() + 1) + '.' + pad(end.getDate()) + '）'
  });
  extra.unshift(coupon);
  wx.setStorageSync(K.extra, extra);
  return coupon;
}
function weekendClaimed() {
  return flag(K.claimed, false);
}
function claimWeekend() {
  if (weekendClaimed()) return null;
  const got = data.WEEKEND_PACK.map(c => grantCoupon(c, 'w'));
  wx.setStorageSync(K.claimed, true);
  return got;
}
function redeem(code) {
  const key = String(code || '').trim().toUpperCase();
  const c = data.CODES[key];
  if (!c) return { ok: false, msg: '兑换码无效或已过期' };
  const done = list(K.codes);
  if (done.indexOf(key) >= 0) return { ok: false, msg: '这个兑换码已经兑换过了' };
  done.push(key);
  wx.setStorageSync(K.codes, done);
  return { ok: true, coupon: grantCoupon(c, 'x') };
}

// ───────── 钱包 ─────────
function getWallet() {
  const w = wx.getStorageSync(K.wallet);
  if (w && typeof w.balance === 'number') return w;
  return {
    balance: 126.5,
    logs: [
      { title: '新人出行礼金', amount: 21.5, type: 'gift', time: '2026-10-08 09:21' },
      { title: '充值 ¥100 赠 ¥5', amount: 105, type: 'recharge', time: '2026-10-08 09:20' }
    ]
  };
}
function changeWallet(amount, title, type) {
  const w = getWallet();
  w.balance = round2(w.balance + amount);
  w.logs.unshift({ title, amount: round2(amount), type, time: dateTime(Date.now()) });
  w.logs = w.logs.slice(0, 50);
  wx.setStorageSync(K.wallet, w);
  return w;
}

// ───────── 行程 ─────────
function seedTrip(s) {
  const from = place(data.findPoi(s.from) || Object.assign({}, data.ORIGIN, { name: s.from }));
  const to = place(data.findPoi(s.to) || { name: s.to });
  const t = Object.assign({ from, to, payBy: 'wechat', couponCut: 0, couponTitle: '' }, s, { from, to });
  if (s.status === 'done') {
    t.fare = data.fare(s.typeKey, s.km, s.min);
    t.pay = round1(t.fare.total - t.couponCut);
  }
  return t;
}
const SEEDS = [
  { id: 'T20261023A1', typeKey: 'express', type: '快车', from: '望京SOHO T1-东门', to: '北京首都国际机场T3航站楼', date: '10月23日 08:12', km: 21.4, min: 36, status: 'done', plate: '京B·Q3M81', driver: '李师傅', car: '大众 朗逸 · 银色', driverPhone: '171 8056 1180', couponCut: 5, couponTitle: '快车券' },
  { id: 'T20261020B7', typeKey: 'premier', type: '专车', from: '国贸三期', to: '三里屯太古里', date: '10月20日 19:46', km: 5.8, min: 22, status: 'done', plate: '京A·N0P66', driver: '赵师傅', car: '奥迪 A6L · 黑色', driverPhone: '171 8056 2207' },
  { id: 'T20261018C2', typeKey: 'pool', type: '拼车', from: '中关村软件园二期', to: '西二旗地铁站', date: '10月18日 18:30', km: 4.1, min: 15, status: 'cancel', cancelReason: '行程有变，暂不需要用车', cancelFee: 0, plate: '-', driver: '-', car: '-' },
  { id: 'T20261012D9', typeKey: 'express', type: '快车', from: '北京南站', to: '五道口购物中心', date: '10月12日 21:05', km: 23.7, min: 48, status: 'done', plate: '京Q·7H2C5', driver: '陈师傅', car: '丰田 卡罗拉 · 白色', driverPhone: '171 8056 3391', couponCut: 5, couponTitle: '快车券' }
];
function getTrips() {
  const v = wx.getStorageSync(K.trips);
  if (Array.isArray(v) && v.length && v[0].from && typeof v[0].from === 'object') return v;
  // 第一次打开：把示例行程写进 storage，之后评价 / 开票 / 投诉都改的是同一份
  const seeds = SEEDS.map(seedTrip);
  wx.setStorageSync(K.trips, seeds);
  return seeds;
}
function findTrip(id) {
  return getTrips().find(t => t.id === id) || null;
}
function addTrip(t) {
  const l = getTrips();
  l.unshift(t);
  wx.setStorageSync(K.trips, l.slice(0, 30));
  return t;
}
function updateTrip(id, patch) {
  const l = getTrips();
  const t = l.find(x => x.id === id);
  if (!t) return null;
  Object.assign(t, patch);
  wx.setStorageSync(K.trips, l);
  return t;
}
function unpaidTrip() {
  return getTrips().find(t => t.status === 'unpaid') || null;
}
// 这一单应付多少：行程费 = 车费 - 券；取消单 = 取消费
function dueOf(t) {
  // 示例里的取消单没有 cancelled 标记、也没有 fare：按状态判，别去读 fare
  if (t.cancelled || t.status === 'cancel') return t.cancelFee || 0;
  return t.fare ? round1(t.fare.total - (t.couponCut || 0)) : 0;
}
// 付款成功：落支付方式、核销券、钱包扣款；行程单变「已完成」，取消单变「已取消」
function payTrip(id, payBy, couponId) {
  const t = findTrip(id);
  if (!t || t.status !== 'unpaid') return null;
  let patch = { payBy, paidAt: Date.now() };
  if (t.cancelled) {
    patch = Object.assign(patch, { status: 'cancel', pay: t.cancelFee });
  } else {
    const c = couponId ? validCoupons().find(x => x.id === couponId) : null;
    const cut = c ? couponCut(c, { typeKey: t.typeKey, from: t.from, to: t.to, amount: t.fare.total, at: t.when || t.createdAt }) : 0;
    patch = Object.assign(patch, { status: 'done', couponId: c && cut ? c.id : '', couponTitle: c && cut ? c.name : '', couponCut: cut, pay: round1(t.fare.total - cut) });
  }
  // 余额不够不扣成负数：页面已经拦过，这里再守一道
  if (payBy === 'wallet' && getWallet().balance < patch.pay) return null;
  if (patch.couponId) useCoupon(patch.couponId);
  if (payBy === 'wallet') changeWallet(-patch.pay, (t.cancelled ? '取消费 ' : '行程支付 ') + t.from.name + ' → ' + t.to.name, 'pay');
  return updateTrip(id, patch);
}
function stats() {
  const all = getTrips();
  const done = all.filter(t => t.status === 'done');
  return {
    count: all.length,
    done: done.length,
    km: round1(done.reduce((s, t) => s + t.km, 0)),
    saved: round1(done.reduce((s, t) => s + (t.couponCut || 0), 0))
  };
}
// 会员等级：按完成的行程数
const LEVELS = [{ name: '白银', n: 0 }, { name: '黄金', n: 4 }, { name: '铂金', n: 10 }, { name: '钻石', n: 30 }];
function levelOf(n) {
  let i = 0;
  LEVELS.forEach((l, k) => { if (n >= l.n) i = k; });
  const top = i === LEVELS.length - 1;
  const next = LEVELS[Math.min(i + 1, LEVELS.length - 1)];
  const cur = LEVELS[i];
  return {
    name: cur.name, next: next.name, top, need: Math.max(0, next.n - n),
    percent: top ? 100 : Math.round(((n - cur.n) / (next.n - cur.n)) * 100)
  };
}

// ───────── 紧急联系人 / 安全设置 ─────────
function getContacts() {
  const v = wx.getStorageSync(K.contacts);
  if (Array.isArray(v)) return v;
  return [{ id: 'p1', name: '林晓雨', phone: '13910862088', relation: '家人' }];
}
function saveContacts(l) {
  wx.setStorageSync(K.contacts, l);
}
function autoShare() {
  return flag(K.autoShare, true);
}
function setAutoShare(v) {
  wx.setStorageSync(K.autoShare, !!v);
}
function recording() {
  return flag(K.record, true);
}
function setRecording(v) {
  wx.setStorageSync(K.record, !!v);
}

// ───────── 发票 ─────────
function getInvoices() {
  return list(K.invoices);
}
function invoiceable() {
  return getTrips().filter(t => t.status === 'done' && t.pay > 0 && !t.invoiceId);
}
function addInvoice(ids, info) {
  const trips = getTrips().filter(t => ids.indexOf(t.id) >= 0 && t.status === 'done' && !t.invoiceId);
  if (!trips.length) return null;
  const inv = Object.assign({}, info, {
    id: 'INV' + Date.now().toString().slice(-8),
    tripIds: trips.map(t => t.id),
    amount: round2(trips.reduce((s, t) => s + t.pay, 0)),
    time: dateTime(Date.now())
  });
  const l = getInvoices();
  l.unshift(inv);
  wx.setStorageSync(K.invoices, l.slice(0, 30));
  trips.forEach(t => updateTrip(t.id, { invoiceId: inv.id }));
  return inv;
}

module.exports = {
  K, round1, round2, pad, hm, dayText, dateTime, maskPhone, isNight,
  place, getFrom, setFrom, getTo, setTo, history, pushHistory, clearHistory, getPlaces, setPlace, removePlace, placesText,
  getType, setType, getProfile, setProfile,
  allCoupons, validCoupons, couponCut, usableCoupons, bestCoupon, getPick, setPick, pickFor, useCoupon, grantCoupon,
  weekendClaimed, claimWeekend, redeem,
  getWallet, changeWallet,
  getTrips, findTrip, addTrip, updateTrip, unpaidTrip, dueOf, payTrip, stats, levelOf,
  getContacts, saveContacts, autoShare, setAutoShare, recording, setRecording,
  getInvoices, invoiceable, addInvoice
};
