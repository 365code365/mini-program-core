// 本地状态：电影票（订单）/ 想看 / 优惠券 / 会员卡 / 影评 / 常去影院，全部存在 storage 里
const D = require('./data.js');
const K = {
  tickets: 'cinema_tickets', wish: 'cinema_wish', pick: 'cinema_pick', vip: 'cinema_vip', card: 'cinema_card',
  used: 'cinema_used_coupons', extra: 'cinema_extra_coupons', reviews: 'cinema_reviews', fav: 'cinema_fav',
  pref: 'cinema_pref', ticketTab: 'cinema_ticket_tab'
};
const MIN = 60000;
const BASE_SEEN = 22; // 在本 App 之前看过的场数（演示数据）
// 一张已经看完的历史票，让「已完成 / 写影评」一开始就有东西
const SEED = [
  { id: 'TK-seed-1', mid: 'm1', cid: 'c3', title: '长河落日', poster: '/images/m1.jpg', day: '10月5日', time: '19:30', end: '21:58', lang: '国语杜比', hall: '杜比厅',
    cinema: '上影国际影城（新天地店）', seats: ['7排8座', '7排9座'], price: 65, code: '58203716', ticketFee: 130, service: 6, snackFee: 0, discount: 0,
    total: 136, payBy: 'wx', status: 'used', ts: 0, phone: '13862216621', createdAt: 0, seed: true }
];
function pad(n) { return (n < 10 ? '0' : '') + n; }
function dateTime(ts) {
  const d = new Date(ts);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}
// getStorageSync 缺省给 ''：数组 / 数字分别兜底，不写 `|| 默认值`
function list(key) {
  const v = wx.getStorageSync(key);
  return Array.isArray(v) ? v : [];
}
// ───────── 想看 ─────────
function wishes() { return list(K.wish); }
function isWished(id) { return wishes().indexOf(id) >= 0; }
function toggleWish(id) {
  const w = wishes();
  const on = w.indexOf(id) < 0;
  wx.setStorageSync(K.wish, on ? w.concat([id]) : w.filter(x => x !== id));
  return on;
}
// ───────── 电影票 ─────────
// 已过散场时间的待观影票算「已观影」
function statusOf(t) {
  if (t.status !== 'unused') return t.status;
  const m = D.getMovie(t.mid);
  return t.ts && Date.now() > t.ts + m.duration * MIN ? 'used' : 'unused';
}
const STATUS_TEXT = { unused: '待观影', used: '已观影', refunded: '已退票' };
function decorate(t) {
  const status = statusOf(t);
  const r = reviewOf(t.id);
  return Object.assign({}, t, {
    status, statusText: STATUS_TEXT[status],
    seatText: (t.seats || []).join(' '),
    codeText: t.code.slice(0, 4) + ' ' + t.code.slice(4),
    reviewed: !!r,
    canRefund: status === 'unused' && t.ts - Date.now() > 60 * MIN,
    canChange: status === 'unused' && !t.changed && t.ts - Date.now() > 60 * MIN,
    payText: t.payBy === 'card' ? '会员卡余额' : '微信支付',
    createdText: t.createdAt ? dateTime(t.createdAt) : '2026-10-03 12:16'
  });
}
function rawTickets() { return list(K.tickets); }
function tickets() { return rawTickets().concat(SEED).map(decorate); }
function ticket(id) {
  const t = rawTickets().concat(SEED).find(x => x.id === id);
  return t ? decorate(t) : null;
}
function updateTicket(id, patch) {
  const l = rawTickets();
  const i = l.findIndex(x => x.id === id);
  if (i < 0) return null;
  l[i] = Object.assign({}, l[i], patch);
  wx.setStorageSync(K.tickets, l);
  return l[i];
}
function seenCount() { return BASE_SEEN + tickets().filter(t => t.status === 'used').length; }
// 下单前算钱：会员免服务费
function quote(pick, couponId, snack) {
  const n = pick.seats.length;
  const ticketFee = pick.price * n;
  const service = isVip() ? 0 : D.SERVICE_FEE * n;
  const c = couponId ? availableCoupons(ticketFee).find(x => x.id === couponId) : null;
  const discount = c ? Math.min(c.off, ticketFee) : 0;
  const snackFee = snack ? D.SNACK_PRICE : 0;
  return { ticketFee, service, snackFee, discount, coupon: c || null, total: ticketFee + service + snackFee - discount };
}
function placeOrder(pick, q, opts) {
  const m = D.getMovie(pick.mid), c = D.getCinema(pick.cid);
  const t = Object.assign({}, pick, q, {
    id: 'TK' + Date.now(), title: m.title, poster: m.poster, cinema: c.name,
    code: String(Math.floor(10000000 + Math.random() * 89999999)),
    snackCode: q.snackFee ? String(Math.floor(1000 + Math.random() * 8999)) : '',
    payBy: opts.payBy, phone: opts.phone, status: 'unused', createdAt: Date.now()
  });
  if (q.coupon) useCoupon(q.coupon.id);
  if (opts.payBy === 'card') cardMove(-q.total, '购票 ' + m.title);
  const l = rawTickets();
  l.unshift(t);
  wx.setStorageSync(K.tickets, l);
  wx.removeStorageSync(K.pick);
  return t;
}
// 退票：每张扣手续费，钱原路退回（会员卡付的退回卡里），券退回
function refund(id) {
  const t = ticket(id);
  if (!t || !t.canRefund) return null;
  const fee = D.REFUND_FEE * t.seats.length;
  const amount = Math.max(0, t.total - fee);
  if (t.coupon) returnCoupon(t.coupon.id);
  if (t.payBy === 'card') cardMove(amount, '退票 ' + t.title);
  updateTicket(id, { status: 'refunded', refund: { amount, fee, ts: Date.now() } });
  return { amount, fee, to: t.payBy === 'card' ? '会员卡' : '微信零钱' };
}
// 改签：同一影院、同一影片的其他场次；差价多退少补，只能改一次
function changeOptions(t) {
  const m = D.getMovie(t.mid), c = D.getCinema(t.cid);
  const out = [];
  for (let d = 0; d < 3; d++) {
    D.shows(m, c, d).forEach(s => {
      if (!s.stop && s.ts - Date.now() > 60 * MIN && s.ts !== t.ts) {
        out.push(Object.assign({ label: D.dayLabel(d) + ' ' + s.time + ' ' + s.lang + ' ¥' + s.price, dayText: D.dayLabel(d) }, s));
      }
    });
  }
  // 离原场次近的排前面，最多 6 个（操作菜单的上限）
  return out.sort((a, b) => Math.abs(a.ts - t.ts) - Math.abs(b.ts - t.ts)).slice(0, 6);
}
function changeTicket(id, s, diff) {
  const t = ticket(id);
  if (!t || !t.canChange) return null;
  if (t.payBy === 'card' && diff !== 0) cardMove(-diff, (diff > 0 ? '改签补差价 ' : '改签退差价 ') + t.title);
  const from = t.day + ' ' + t.time;
  return updateTicket(id, {
    day: s.dayText, time: s.time, end: s.end, lang: s.lang, hall: s.hall, price: s.price, ts: s.ts,
    ticketFee: s.price * t.seats.length, total: t.total + diff, changed: { from, diff, ts: Date.now() }
  });
}
// ───────── 优惠券 ─────────
function allCoupons() { return D.COUPONS.concat(list(K.extra)); }
function usedIds() { return list(K.used); }
function availableCoupons(amount) {
  const used = usedIds();
  return allCoupons().filter(c => used.indexOf(c.id) < 0 && (amount === undefined || c.min <= amount));
}
function useCoupon(id) { wx.setStorageSync(K.used, usedIds().concat([id])); }
function returnCoupon(id) { wx.setStorageSync(K.used, usedIds().filter(x => x !== id)); }
function couponLists() {
  const used = usedIds();
  return { usable: allCoupons().filter(c => used.indexOf(c.id) < 0), used: allCoupons().filter(c => used.indexOf(c.id) >= 0) };
}
// 兑换码：返回 { ok, msg }
function redeem(code) {
  const c = D.CODES[String(code || '').trim().toUpperCase()];
  if (!c) return { ok: false, msg: '兑换码无效' };
  if (allCoupons().some(x => x.id === c.id)) return { ok: false, msg: '这个兑换码已经用过了' };
  wx.setStorageSync(K.extra, list(K.extra).concat([c]));
  return { ok: true, msg: '已领取「' + c.name + '」', coupon: c };
}
// ───────── 会员卡 ─────────
function isVip() { return !!wx.getStorageSync(K.vip); }
function card() {
  const v = wx.getStorageSync(K.card);
  return v && typeof v.balance === 'number' ? v : { balance: 0, logs: [] };
}
function cardMove(amount, title) {
  const c = card();
  c.balance = Math.round((c.balance + amount) * 100) / 100;
  c.logs = [{ title, amount, ts: Date.now() }].concat(c.logs).slice(0, 50);
  wx.setStorageSync(K.card, c);
  return c;
}
function openVip() { wx.setStorageSync(K.vip, true); }
function recharge(opt) { return cardMove(opt.amount + opt.gift, '充值 ¥' + opt.amount + (opt.gift ? '（赠 ¥' + opt.gift + '）' : '')); }
// ───────── 影评 ─────────
function reviews() { return list(K.reviews); }
function reviewOf(tid) { return reviews().find(r => r.tid === tid) || null; }
function reviewById(id) { return reviews().find(r => r.id === id) || null; }
function saveReview(r) {
  const l = reviews();
  const i = l.findIndex(x => x.id === r.id);
  if (i >= 0) l[i] = Object.assign({}, l[i], r, { ts: Date.now() });
  else l.unshift(Object.assign({ id: 'RV' + Date.now(), likes: 0 }, r, { ts: Date.now() }));
  wx.setStorageSync(K.reviews, l);
  return l;
}
function removeReview(id) { wx.setStorageSync(K.reviews, reviews().filter(r => r.id !== id)); }
// ───────── 常去影院 / 偏好 ─────────
function favCinema() { return wx.getStorageSync(K.fav) || 'c1'; }
function setFavCinema(id) { wx.setStorageSync(K.fav, id); }
function pref() { return wx.getStorageSync(K.pref) || 'IMAX · 中间排'; }
function setPref(p) { wx.setStorageSync(K.pref, p); }
module.exports = {
  K, dateTime, wishes, isWished, toggleWish,
  tickets, ticket, seenCount, quote, placeOrder, refund, changeOptions, changeTicket,
  availableCoupons, couponLists, redeem,
  isVip, card, openVip, recharge,
  reviews, reviewOf, reviewById, saveReview, removeReview,
  favCinema, setFavCinema, pref, setPref
};
