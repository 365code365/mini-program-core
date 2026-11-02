// 本地状态：搜索条件 / 订单 / 常用入住人 / 发票抬头 / 优惠券 / 积分 / 收藏与浏览，全部存在 storage 里
const data = require('./data.js');
const HOUR = 3600000;
const DAY = 24 * HOUR;
const K = {
  search: 'hotel_search', orders: 'hotel_orders', guests: 'hotel_guests', guestPick: 'hotel_guest_pick', contact: 'hotel_guest',
  invoices: 'hotel_invoices', invPick: 'hotel_inv_pick', used: 'hotel_used_coupons', points: 'hotel_points', pointsLog: 'hotel_points_log',
  signed: 'hotel_signed', favs: 'hotel_favs', views: 'hotel_views', orderTab: 'hotel_order_tab'
};
function pad(n) { return n < 10 ? '0' + n : '' + n; }
function round2(n) { return Math.round(n * 100) / 100; }
function fmt(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function parse(s) { const p = s.split('-'); return new Date(+p[0], +p[1] - 1, +p[2]); }
function addDays(s, n) { const d = parse(s); return fmt(new Date(d.getFullYear(), d.getMonth(), d.getDate() + n)); }
function today() { return fmt(new Date()); }
function short(s) { const d = parse(s); return (d.getMonth() + 1) + '月' + d.getDate() + '日'; }
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
function week(s) { return WEEK[parse(s).getDay()]; }
function nights(a, b) { return Math.max(1, Math.round((parse(b) - parse(a)) / DAY)); }
function ts(date, hm) { const d = parse(date); const h = hm.split(':').map(Number); return new Date(d.getFullYear(), d.getMonth(), d.getDate(), h[0], h[1] || 0).getTime(); }
function dateTime(t) { const d = new Date(t); return fmt(d) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes()); }
function list(key) { const v = wx.getStorageSync(key); return Array.isArray(v) ? v : null; }
function num(key, def) { const v = wx.getStorageSync(key); return typeof v === 'number' ? v : def; }
function maskPhone(p) { return p && p.length === 11 ? p.slice(0, 3) + '****' + p.slice(7) : p; }

// ───────── 搜索条件 ─────────
function getSearch() {
  const t = today();
  const def = { city: '杭州', checkin: t, checkout: addDays(t, 1), keyword: '', price: 0, star: 0, kind: 0, rooms: 1, adults: 2, children: 0 };
  const out = Object.assign(def, wx.getStorageSync(K.search) || {});
  if (out.checkin < t) { out.checkin = t; out.checkout = addDays(t, 1); }
  if (out.checkout <= out.checkin) out.checkout = addDays(out.checkin, 1);
  return out;
}
function saveSearch(s) { wx.setStorageSync(K.search, s); }
function peopleText(s) { return s.rooms + '间 ' + s.adults + '成人' + (s.children ? ' ' + s.children + '儿童' : ''); }

// ───────── 收藏 / 浏览 ─────────
function favs() { return list(K.favs) || []; }
function isFav(id) { return favs().indexOf(id) >= 0; }
function toggleFav(id) {
  const l = favs();
  const on = l.indexOf(id) < 0;
  wx.setStorageSync(K.favs, on ? [id].concat(l) : l.filter(x => x !== id));
  return on;
}
function views() { return list(K.views) || []; }
function addView(id) { wx.setStorageSync(K.views, [id].concat(views().filter(x => x !== id)).slice(0, 20)); }
function clearViews() { wx.setStorageSync(K.views, []); }

// ───────── 会员与积分：金卡 9.5 折，住满 20 晚升铂金卡 ─────────
const BASE_NIGHTS = 17;
function member() {
  const n = BASE_NIGHTS + getOrders().filter(o => !o.seed && stageOf(o).key === 'done').reduce((s, o) => s + (o.hourly ? 0 : o.nights * o.count), 0);
  return { name: n >= 20 ? '铂金卡会员' : '金卡会员', rate: 0.95, nights: n, need: Math.max(0, 20 - n), pct: Math.min(100, Math.round(n / 20 * 100)) };
}
function getPoints() { return num(K.points, 1860); }
function pointsLog() {
  return list(K.pointsLog) || [{ title: '历史住店累计', delta: 1360, time: '2026-06-01 10:00' }, { title: '金卡开卡礼', delta: 500, time: '2026-01-08 09:30' }];
}
function changePoints(delta, title) {
  const p = Math.max(0, getPoints() + delta);
  wx.setStorageSync(K.points, p);
  const log = pointsLog();
  log.unshift({ title, delta, time: dateTime(Date.now()) });
  wx.setStorageSync(K.pointsLog, log.slice(0, 50));
  return p;
}
function signedToday() { return wx.getStorageSync(K.signed) === today(); }
function sign() {
  if (signedToday()) return false;
  wx.setStorageSync(K.signed, today());
  changePoints(10, '每日签到');
  return true;
}

// ───────── 优惠券 ─────────
function allCoupons() {
  const used = list(K.used) || [];
  return data.COUPONS.map(c => Object.assign({}, c, { expire: addDays(today(), c.days), status: c.days < 0 ? 'expired' : (used.indexOf(c.id) >= 0 ? 'used' : 'valid') }));
}
function validCoupons() { return allCoupons().filter(c => c.status === 'valid'); }
function couponCut(c, fee, h, room) {
  if (!c || fee < c.min) return 0;
  if (c.type && h.type !== c.type) return 0;
  // 钟点房券只用于钟点房，其余券不用于钟点房
  if (!!c.hourly !== !!room.hours) return 0;
  return Math.min(c.amount, fee);
}
function useCoupon(id) { const u = list(K.used) || []; if (u.indexOf(id) < 0) u.push(id); wx.setStorageSync(K.used, u); }
function restoreCoupon(id) { wx.setStorageSync(K.used, (list(K.used) || []).filter(x => x !== id)); }
// 价格：房费 → 会员 9.5 折 → 优惠券 → 积分（100 抵 1 元，最多 50 元且不超过房费 10%）
function price(h, room, count, n, couponId, usePoints) {
  const roomFee = room.price * count * (room.hours ? 1 : n);
  const memberCut = Math.round(roomFee * (1 - member().rate));
  const valid = validCoupons();
  let coupon = null;
  if (couponId === undefined) valid.forEach(c => { if (couponCut(c, roomFee, h, room) > couponCut(coupon, roomFee, h, room)) coupon = c; });
  else coupon = valid.find(c => c.id === couponId) || null;
  const couponCutV = couponCut(coupon, roomFee, h, room);
  const left = roomFee - memberCut - couponCutV;
  const pointsMax = Math.max(0, Math.min(Math.floor(getPoints() / 100), 50, Math.floor(roomFee * 0.1), Math.floor(left - 1)));
  const pointsCut = usePoints ? pointsMax : 0;
  return {
    roomFee, memberCut, coupon: couponCutV ? coupon : null, couponCut: couponCutV, pointsMax, pointsCut,
    usable: valid.filter(c => couponCut(c, roomFee, h, room) > 0).length,
    total: Math.max(0.01, round2(left - pointsCut)), saved: memberCut + couponCutV + pointsCut
  };
}

// ───────── 订单：状态按入住 / 离店时间推进 ─────────
const SEEDS = () => {
  const t = today();
  return [
    { id: 'HT20260915', seed: true, hid: 'h5', rid: 'r1', count: 1, checkin: addDays(t, -48), checkout: addDays(t, -46), nights: 2, guests: ['林晓舟'], phone: '13800006621',
      arrival: '18:00-20:00', arrivalStart: '18:00', needs: '安静房', invoice: null, roomFee: 1256, memberCut: 63, couponCut: 0, pointsCut: 0, total: 1193, createdAt: parse(addDays(t, -55)).getTime() + 10 * HOUR, status: 'paid' },
    { id: 'HT20260802', seed: true, hid: 'h2', rid: 'r1', count: 1, checkin: addDays(t, -92), checkout: addDays(t, -91), nights: 1, guests: ['林晓舟'], phone: '13800006621',
      arrival: '14:00-16:00', arrivalStart: '14:00', needs: '', invoice: null, roomFee: 698, memberCut: 35, couponCut: 0, pointsCut: 0, total: 663, createdAt: parse(addDays(t, -96)).getTime() + 9 * HOUR, status: 'cancel', refund: 663 }
  ];
};
function getOrders() {
  const v = list(K.orders);
  if (v) return v;
  const s = SEEDS();
  wx.setStorageSync(K.orders, s);
  return s;
}
function saveOrders(l) { wx.setStorageSync(K.orders, l); }
function findOrder(id) { return getOrders().find(o => o.id === id) || null; }
function addOrder(o) { saveOrders([o].concat(getOrders()).slice(0, 50)); }
function updateOrder(id, patch) {
  const l = getOrders();
  const o = l.find(x => x.id === id);
  if (!o) return null;
  Object.assign(o, patch);
  saveOrders(l);
  return o;
}
// 入住时刻按预计到店时间算，离店按 12:00；钟点房从开始时间算 hours 小时
function stayRange(o) {
  if (o.hourly) { const s = ts(o.checkin, o.start); return [s, s + o.hours * HOUR]; }
  // 当天下单、到店时段已开始的：给 1 小时赶到酒店，期间仍是「待入住」（可改到店时间、30 分钟内可免费取消）
  return [Math.max(ts(o.checkin, o.arrivalStart || '14:00'), (o.createdAt || 0) + HOUR), ts(o.checkout, '12:00')];
}
// 免费取消：入住前一天 18:00 前；当天的订单下单后 30 分钟内；钟点房开始前
function freeCancel(o, now) {
  if (o.hourly) return now < ts(o.checkin, o.start);
  return now < ts(addDays(o.checkin, -1), '18:00') || now < o.createdAt + 30 * 60000;
}
const ST = { wait: '待入住', in: '入住中', done: '已完成', cancel: '已取消' };
function stageOf(o, now) {
  now = now || Date.now();
  if (o.status === 'cancel') return { key: 'cancel', text: ST.cancel, tip: '订单已取消，¥' + (o.refund !== undefined ? o.refund : o.total) + ' 已原路退回', canCancel: false };
  const r = stayRange(o);
  let key = 'wait';
  if (now >= r[0]) key = 'in';
  if (now >= r[1]) key = 'done';
  const free = key === 'wait' && freeCancel(o, now);
  const tips = {
    wait: free ? '酒店已确认，房间为你保留。现在取消可全额退款' : '酒店已确认，房间为你保留。现在取消将扣除首晚房费',
    in: '入住愉快！离店时间为 ' + (o.hourly ? dateTime(r[1]).slice(11) : short(o.checkout) + ' 12:00 前'),
    done: o.rating ? '感谢入住，期待再次光临' : '已离店，来说说这次住得怎么样吧'
  };
  return { key, text: ST[key], tip: tips[key], canCancel: key === 'wait', free, canRate: key === 'done' && !o.rating, canArrival: key === 'wait' && !o.hourly };
}
// 取消：免费期内全额退，超过时间扣首晚房费；券、积分原路退回
function cancelOrder(id) {
  const o = findOrder(id);
  const st = o && stageOf(o);
  if (!st || !st.canCancel) return null;
  const firstNight = o.hourly ? o.total : Math.min(o.total, Math.round(o.total / o.nights));
  const refund = st.free ? o.total : round2(o.total - firstNight);
  if (o.couponId) restoreCoupon(o.couponId);
  if (o.pointsUsed) changePoints(o.pointsUsed, '订单取消退回 ' + o.id);
  if (o.earn) changePoints(-o.earn, '订单取消扣回 ' + o.id);
  return updateOrder(id, { status: 'cancel', refund, cancelledAt: Date.now() });
}

// ───────── 常用入住人 / 发票抬头 ─────────
function getGuests() { return list(K.guests) || [{ id: 'g1', name: '林晓舟', phone: '13800006621' }, { id: 'g2', name: '林小满', phone: '' }]; }
function saveGuests(l) { wx.setStorageSync(K.guests, l); }
function upsertGuest(g) {
  const l = getGuests();
  if (!g.id) { g.id = 'g' + Date.now().toString(36); l.unshift(g); }
  else { const i = l.findIndex(x => x.id === g.id); if (i >= 0) l[i] = g; else l.unshift(g); }
  saveGuests(l);
  return g;
}
function removeGuest(id) { saveGuests(getGuests().filter(x => x.id !== id)); }
// 下单时填的新名字记进常用入住人
function rememberGuests(names) {
  const l = getGuests();
  names.forEach(n => { if (n && !l.some(x => x.name === n)) l.push({ id: 'g' + Date.now().toString(36) + l.length, name: n, phone: '' }); });
  saveGuests(l);
}
function getInvoices() { return list(K.invoices) || [{ id: 'v1', type: 1, title: '杭州途栖网络科技有限公司', taxNo: '91330106MA2H8K6Q3X' }]; }
function saveInvoices(l) { wx.setStorageSync(K.invoices, l); }
function upsertInvoice(v) {
  const l = getInvoices();
  if (!v.id) { v.id = 'v' + Date.now().toString(36); l.unshift(v); }
  else { const i = l.findIndex(x => x.id === v.id); if (i >= 0) l[i] = v; else l.unshift(v); }
  saveInvoices(l);
  return v;
}
function removeInvoice(id) { saveInvoices(getInvoices().filter(x => x.id !== id)); }

// ───────── 点评：示例点评 + 自己评价过的订单 ─────────
function reviewsOf(h) {
  const mine = getOrders().filter(o => o.hid === h.id && o.rating).map(o => ({
    k: o.id, user: '我', avatar: '我', mine: true, score: o.rating.score, text: o.rating.text || '（未填写文字）', tags: o.rating.tags,
    room: (h.rooms.find(r => r.id === o.rid) || h.rooms[0]).name, date: o.rating.time.slice(0, 10)
  }));
  const t = today();
  return mine.concat(data.seedReviews(h).map(r => Object.assign({ date: addDays(t, -r.ago) }, r)));
}

module.exports = {
  K, HOUR, DAY, pad, round2, fmt, parse, addDays, today, short, week, nights, ts, dateTime, maskPhone,
  getSearch, saveSearch, peopleText, favs, isFav, toggleFav, views, addView, clearViews,
  member, getPoints, pointsLog, changePoints, signedToday, sign,
  allCoupons, validCoupons, couponCut, useCoupon, restoreCoupon, price,
  getOrders, findOrder, addOrder, updateOrder, stageOf, cancelOrder, stayRange,
  getGuests, upsertGuest, removeGuest, rememberGuests, getInvoices, upsertInvoice, removeInvoice, reviewsOf
};
