// 本地状态：购物车 / 结算单 / 订单 / 收花地址 / 优惠券 / 纪念日 / 会员，全部存在 storage 里
const data = require('./data.js');
const HOUR = 3600000;
const DAY = 24 * HOUR;
const K = {
  cart: 'fl_cart', orders: 'fl_orders', addrs: 'fl_addrs', addrPick: 'fl_addr_pick', used: 'fl_used_coupons',
  dates: 'fl_dates', orderTab: 'fl_order_tab', usePick: 'fl_use_pick'
};
function pad(v) { return v < 10 ? '0' + v : '' + v; }
function round2(n) { return Math.round(n * 100) / 100; }
function dateStr(ts) {
  const d = new Date(ts);
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate());
}
function dateTime(ts) {
  const d = new Date(ts);
  return dateStr(ts) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes());
}
function dayAfter(n) {
  const t = new Date();
  return dateStr(new Date(t.getFullYear(), t.getMonth(), t.getDate() + n).getTime());
}
// 'YYYY-MM-DD' + 'HH:MM' → 时间戳（手动拆，不依赖 Date 的字符串解析）
function ts(date, hm) {
  const p = date.split('-').map(Number);
  const h = (hm || '00:00').split(':').map(Number);
  return new Date(p[0], p[1] - 1, p[2], h[0], h[1] || 0).getTime();
}
function list(key) {
  const v = wx.getStorageSync(key);
  return Array.isArray(v) ? v : null;
}
function clone(v) { return JSON.parse(JSON.stringify(v)); }
function maskPhone(p) { return p && p.length === 11 ? p.slice(0, 3) + '****' + p.slice(7) : p; }

// ───────── 配送时段 ─────────
// 同城 3 小时达：当天的时段要在结束前至少还有 3 小时才来得及送
function slotOpen(date, slot, now) {
  return ts(date, slot.split('-')[1]) - (now || Date.now()) >= 3 * HOUR;
}
// 选了今天、而所选时段来不及：换成今天最早还能送的时段，今天都不行就改到明天
function fixSlot(date, slot, now) {
  if (slotOpen(date, slot, now)) return { date, slot, changed: false };
  const ok = data.slots.find(s => slotOpen(date, s, now));
  if (ok) return { date, slot: ok, changed: true };
  return { date: dateStr(ts(date) + DAY), slot: data.slots[0], changed: true };
}

// ───────── 购物车 ─────────
const DEFAULT_CART = [
  { key: 'f4-11-mist', id: 'f4', stems: 11, pack: 'mist', date: dayAfter(1), slot: '15:00-18:00', card: '生日快乐，愿你永远热烈灿烂。', qty: 1, checked: true }
];
function getCart() { return list(K.cart) || clone(DEFAULT_CART); }
function saveCart(v) { wx.setStorageSync(K.cart, v); }
function cartCount() { return getCart().reduce((s, x) => s + x.qty, 0); }
function addToCart(item) {
  const cart = getCart();
  const key = item.id + '-' + item.stems + '-' + item.pack;
  const hit = cart.find(x => x.key === key);
  if (hit) { hit.qty = Math.min(99, hit.qty + item.qty); Object.assign(hit, { date: item.date, slot: item.slot, card: item.card }); }
  else cart.unshift(Object.assign({ key, checked: true }, item));
  saveCart(cart);
}
// 把一行（id/stems/pack/qty）展开成带名称、图片、单价的视图数据
function expand(line) {
  const p = data.productOf(line.id);
  const unit = data.unitPrice(p, line.stems, line.pack);
  return Object.assign({}, line, { name: p.name, img: p.img, use: p.use, unit, sum: unit * line.qty, half: p.id === data.HALF_ID,
    spec: data.stemOf(line.stems).name + ' · ' + data.packOf(line.pack).name });
}

// ───────── 会员：累计消费 1000 元升金卡（9 折），之前是银卡（9.5 折）；1 元 1 积分 ─────────
const BASE_SPENT = 680;
const BASE_POINTS = 1280;
function myOrders() { return getOrders().filter(o => !o.seed && o.status !== 'cancel'); }
function member() {
  const spent = round2(BASE_SPENT + myOrders().reduce((s, o) => s + o.total, 0));
  const gold = spent >= 1000;
  return { name: gold ? '金卡会员' : '银卡会员', short: gold ? '金卡' : '银卡', rate: gold ? 0.9 : 0.95, rateText: gold ? '9 折' : '9.5 折',
    gold, spent, need: gold ? 0 : round2(1000 - spent) };
}
function points() { return BASE_POINTS + myOrders().reduce((s, o) => s + Math.floor(o.total), 0); }

// ───────── 优惠券 ─────────
function allCoupons() {
  const used = list(K.used) || [];
  return data.coupons.map(c => Object.assign({}, c, {
    expire: dayAfter(c.days),
    status: c.days < 0 ? 'expired' : (used.indexOf(c.id) >= 0 ? 'used' : 'valid')
  }));
}
function validCoupons() { return allCoupons().filter(c => c.status === 'valid'); }
function useCoupon(id) {
  const used = list(K.used) || [];
  if (id && used.indexOf(id) < 0) used.push(id);
  wx.setStorageSync(K.used, used);
}
function restoreCoupon(id) { wx.setStorageSync(K.used, (list(K.used) || []).filter(x => x !== id)); }
// 一张券在这一单上能省多少。门槛看花束金额（活动之后、会员折扣之前；限用途的券只算该用途的花束），
// 减的钱从 p.base（活动与会员折扣之后的金额）里扣
function couponCut(c, lines, p) {
  if (!c || p.base <= 0) return 0;
  const scope = c.use ? lines.filter(x => x.use === c.use).reduce((s, x) => s + x.sum, 0) : p.goods - p.activityCut;
  if (scope < c.min) return 0;
  return Math.min(c.amount, p.base);
}
// 价格：花束 → 向日葵第二束半价 → 会员折扣 → 优惠券。couponId：undefined 自动挑最省的，'' 不用券
function price(lines, couponId) {
  const goods = lines.reduce((s, x) => s + x.sum, 0);
  const halfUnits = [];
  lines.filter(x => x.half).forEach(x => { for (let i = 0; i < x.qty; i++) halfUnits.push(x.unit); });
  halfUnits.sort((a, b) => b - a);
  let activityCut = 0;
  for (let i = 1; i < halfUnits.length; i += 2) activityCut += halfUnits[i] / 2;
  activityCut = round2(activityCut);
  const m = member();
  const memberCut = round2((goods - activityCut) * (1 - m.rate));
  const base = round2(goods - activityCut - memberCut);
  const q = { goods, activityCut, base };
  const valid = validCoupons();
  let coupon = null;
  if (couponId === undefined) valid.forEach(c => { if (couponCut(c, lines, q) > couponCut(coupon, lines, q)) coupon = c; });
  else coupon = valid.find(c => c.id === couponId) || null;
  const cut = couponCut(coupon, lines, q);
  const usable = valid.filter(c => couponCut(c, lines, q) > 0).length;
  return {
    goods, activityCut, memberCut, memberName: m.name, memberRate: m.rateText, base,
    coupon: cut ? coupon : null, couponCut: cut, usable,
    total: Math.max(0.01, round2(base - cut))
  };
}

// ───────── 订单：状态按送达时段推进 ─────────
// 下单 → 花艺制作（时段开始前 2 小时开工，之前可取消；当天急单支付后 10 分钟内也可取消）→ 配送中（时段开始前 1 小时出发）→ 已签收（时段结束）
const FLOW = ['paid', 'shipping', 'done'];
const STATUS = { paid: '花艺制作中', shipping: '配送中', done: '已签收', cancel: '已取消' };
function noOf(n, hm) { return 'HY' + dayAfter(n).replace(/-/g, '') + hm; }
const DEFAULT_ORDERS = [
  { no: noOf(0, '100315'), seed: true, status: 'shipping', time: dayAfter(0) + ' 10:03', date: dayAfter(0), slot: '18:00-21:00', receiver: '苏晚晴', phone: '139****2086',
    addr: '上海市徐汇区漕溪北路 88 号 圣爱大厦 12 楼', anonymous: true, sender: '', callFirst: false, goods: 259, memberCut: 0, couponCut: 0, activityCut: 0, total: 259,
    items: [{ id: 'f1', stems: 19, pack: 'mist', qty: 1, card: '你是我平淡生活里的浪漫。' }] },
  { no: noOf(-12, '091142'), seed: true, status: 'done', time: dayAfter(-12) + ' 09:11', date: dayAfter(-11), slot: '09:00-12:00', receiver: '王秀兰', phone: '137****5521',
    addr: '杭州市西湖区文三路 259 号 昌地火炬大厦', anonymous: false, sender: '小林', callFirst: true, goods: 209, memberCut: 0, couponCut: 0, activityCut: 0, total: 209,
    items: [{ id: 'f6', stems: 11, pack: 'mist', qty: 1, card: '祝您早日康复，平安顺遂。' }] }
];
function timeStage(o, now) {
  const start = ts(o.date, o.slot.split('-')[0]);
  const end = ts(o.date, o.slot.split('-')[1]);
  if (now >= end) return 'done';
  if (now >= Math.max(start - HOUR, (o.createdAt || 0) + HOUR)) return 'shipping';
  return 'paid';
}
// 只往前推（示例里的「配送中」订单不会因为时间早而退回制作中）
function advance(o, now) {
  if (o.status === 'cancel') return o.status;
  const k = timeStage(o, now);
  return FLOW.indexOf(k) > FLOW.indexOf(o.status) ? k : o.status;
}
function getOrders() {
  const l = list(K.orders) || clone(DEFAULT_ORDERS);
  const now = Date.now();
  let changed = !list(K.orders);
  l.forEach(o => {
    const s = advance(o, now);
    if (s !== o.status) { o.status = s; changed = true; }
  });
  if (changed) wx.setStorageSync(K.orders, l);
  return l;
}
function saveOrders(v) { wx.setStorageSync(K.orders, v); }
function findOrder(no) { return getOrders().find(o => o.no === no) || null; }
function addOrder(o) { saveOrders([o].concat(getOrders()).slice(0, 50)); }
function updateOrder(no, patch) {
  const l = getOrders();
  const o = l.find(x => x.no === no);
  if (!o) return null;
  Object.assign(o, patch);
  saveOrders(l);
  return o;
}
function stageOf(o, now) {
  now = now || Date.now();
  const st = o.status;
  const start = ts(o.date, o.slot.split('-')[0]);
  const canCancel = st === 'paid' && (now < start - 2 * HOUR || now < (o.createdAt || 0) + 10 * 60000);
  const tips = {
    paid: canCancel ? '花艺师会在送达前 2 小时开始制作，制作前可免费取消' : '花艺师正在为你精心制作花束',
    shipping: '配送员' + data.COURIER.name + '正在配送，预计 ' + o.slot + ' 送达',
    done: '已签收，愿 TA 收到花的那一刻很开心',
    cancel: '订单已取消，款项与优惠券已原路退回'
  };
  return { status: st, text: STATUS[st], tip: tips[st], step: { paid: 1, shipping: 2, done: 3 }[st] || 0,
    canCancel, canRate: st === 'done' && !o.rating, active: st === 'paid' || st === 'shipping' };
}
// 取消：券退回，钱原路退回微信
function cancelOrder(no) {
  const o = findOrder(no);
  if (!o || !stageOf(o).canCancel) return null;
  if (o.couponId) restoreCoupon(o.couponId);
  return updateOrder(no, { status: 'cancel', cancelledAt: Date.now() });
}
// 再来一单：原规格、原贺卡加回购物车，送达日期改成明天
function reorder(o) {
  o.items.forEach(x => { if (data.productOf(x.id)) addToCart({ id: x.id, stems: x.stems, pack: x.pack, qty: x.qty, date: dayAfter(1), slot: o.slot, card: x.card || '' }); });
}

// ───────── 收花地址 ─────────
const DEFAULT_ADDRS = [
  { id: 'a1', name: '苏晚晴', phone: '13917522086', region: ['上海市', '上海市', '徐汇区'], detail: '漕溪北路 88 号 圣爱大厦 12 楼', tag: '公司', isDefault: true },
  { id: 'a2', name: '王秀兰', phone: '13757365521', region: ['浙江省', '杭州市', '西湖区'], detail: '文三路 259 号 昌地火炬大厦 3 单元', tag: '家', isDefault: false }
];
function getAddrs() { return list(K.addrs) || clone(DEFAULT_ADDRS); }
function saveAddrs(l) { wx.setStorageSync(K.addrs, l); }
function addrText(a) {
  const r = a.region || [];
  return (r[0] === r[1] ? r[0] + (r[2] || '') : r.join('')) + a.detail;
}
function upsertAddr(a) {
  let l = getAddrs();
  if (!a.id) a.id = 'a' + Date.now().toString(36);
  if (a.isDefault || !l.some(x => x.id !== a.id)) {
    l = l.map(x => Object.assign({}, x, { isDefault: false }));
    a.isDefault = true;
  }
  const i = l.findIndex(x => x.id === a.id);
  if (i >= 0) l[i] = a; else l.unshift(a);
  saveAddrs(l);
  return a;
}
function removeAddr(id) {
  const l = getAddrs().filter(x => x.id !== id);
  if (l.length && !l.some(x => x.isDefault)) l[0].isDefault = true;
  saveAddrs(l);
  if (wx.getStorageSync(K.addrPick) === id) wx.removeStorageSync(K.addrPick);
}
// 这一单送到哪：先看结算页挑过的，再看默认地址
function currentAddr() {
  const l = getAddrs();
  const id = wx.getStorageSync(K.addrPick);
  return l.find(x => x.id === id) || l.find(x => x.isDefault) || l[0] || null;
}
function pickAddr(id) { wx.setStorageSync(K.addrPick, id); }

// ───────── 纪念日：只存月-日，每年循环；离下一次还有几天按今天算 ─────────
function mdAfter(n) { return dayAfter(n).slice(5); }
function defaultDates() {
  return [
    { id: 'd1', name: '妈妈生日', md: mdAfter(5), kind: 'birthday' },
    { id: 'd2', name: '结婚纪念日', md: mdAfter(48), kind: 'love' }
  ];
}
function getDates() {
  const v = list(K.dates);
  if (v) return v;
  const d = defaultDates();
  wx.setStorageSync(K.dates, d);
  return d;
}
function daysLeft(md, now) {
  const t = new Date(now || Date.now());
  const today = new Date(t.getFullYear(), t.getMonth(), t.getDate()).getTime();
  const p = md.split('-').map(Number);
  let next = new Date(t.getFullYear(), p[0] - 1, p[1]).getTime();
  if (next < today) next = new Date(t.getFullYear() + 1, p[0] - 1, p[1]).getTime();
  return Math.round((next - today) / DAY);
}
function datesView() {
  return getDates().map(d => {
    const u = data.useOf(d.kind);
    const p = d.md.split('-');
    return Object.assign({}, d, { left: daysLeft(d.md), text: Number(p[0]) + ' 月 ' + Number(p[1]) + ' 日', color: u.color, useName: u.name });
  }).sort((a, b) => a.left - b.left);
}
function upsertDate(d) {
  const l = getDates();
  if (!d.id) { d.id = 'd' + Date.now().toString(36); l.push(d); }
  else { const i = l.findIndex(x => x.id === d.id); if (i >= 0) l[i] = d; else l.push(d); }
  wx.setStorageSync(K.dates, l);
  return d;
}
function removeDate(id) { wx.setStorageSync(K.dates, getDates().filter(x => x.id !== id)); }

module.exports = {
  K, HOUR, DAY, pad, round2, dateStr, dateTime, dayAfter, ts, maskPhone, slotOpen, fixSlot,
  getCart, saveCart, cartCount, addToCart, expand,
  member, points, allCoupons, validCoupons, useCoupon, restoreCoupon, couponCut, price,
  STATUS, getOrders, saveOrders, findOrder, addOrder, updateOrder, stageOf, cancelOrder, reorder,
  getAddrs, addrText, upsertAddr, removeAddr, currentAddr, pickAddr,
  getDates, datesView, upsertDate, removeDate, daysLeft
};
