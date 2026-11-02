// 鲜到家超市的订单：提交 → 待付款（15 分钟内付）→ 付款后按时间推进 拣货 → 配送 → 送达；
// 取消、售后的钱原路退回（余额付的退回余额），锁住的券与鲜豆一并退回
const data = require('./data.js');
const store = require('./store.js');
const KEY = 'fm_orders';
const MIN = store.MIN;
const PAY_WINDOW = 15 * MIN;
const PICK_MIN = 3;
const ARRIVE_MIN = 30;
const AFTER_DAYS = 7;
const STEPS = ['已支付', '拣货中', '配送中', '已送达'];
const RIDER = { name: '王师傅', phone: '137 0013 8421' };
const TABS = [
  { key: 'all', name: '全部' },
  { key: 'unpaid', name: '待付款' },
  { key: 'receiving', name: '待收货' },
  { key: 'rate', name: '待评价' },
  { key: 'after', name: '退款售后' }
];

function raw() {
  const v = wx.getStorageSync(KEY);
  return Array.isArray(v) ? v : [];
}
function save(l) {
  wx.setStorageSync(KEY, l.slice(0, 40));
}
function short(id) {
  return id.slice(-6);
}
function mmss(ms) {
  const s = Math.ceil(ms / 1000);
  return `${store.pad(Math.floor(s / 60))}:${store.pad(s % 60)}`;
}
// 提交订单时锁住的券、鲜豆：取消 / 超时关闭时退回
function release(o) {
  if (o.couponId) store.restoreCoupon(o.couponId);
  if (o.beansUsed) store.changeBeans(o.beansUsed, `订单取消退回 ${short(o.id)}`);
}
// 待付款超过 15 分钟自动关闭（读订单时顺手结算）
function getOrders() {
  const l = raw();
  const now = Date.now();
  let dirty = false;
  l.forEach(o => {
    if (o.status === 'unpaid' && now >= o.payExpire) {
      Object.assign(o, { status: 'cancelled', cancelledAt: o.payExpire, refund: 0, cancelReason: '超时未支付，订单已自动关闭，优惠券与鲜豆已退回' });
      release(o);
      dirty = true;
    }
  });
  if (dirty) save(l);
  return l;
}
function findOrder(id) {
  return getOrders().find(o => o.id === id) || null;
}
function updateOrder(id, patch) {
  const l = raw();
  const o = l.find(x => x.id === id);
  if (!o) return null;
  Object.assign(o, patch);
  save(l);
  return o;
}

// 提交订单：先落一张待付款的单，券与鲜豆马上锁住
function create(fields) {
  const now = Date.now();
  const o = Object.assign({ id: 'FM' + now.toString().slice(-10), createdAt: now, status: 'unpaid', payExpire: now + PAY_WINDOW }, fields);
  const l = raw();
  l.unshift(o);
  save(l);
  if (o.couponId) store.useCoupon(o.couponId);
  if (o.beansUsed) store.changeBeans(-o.beansUsed, `下单抵扣 ${short(o.id)}`);
  return o;
}
// 付款成功：余额支付在这里扣钱（不够返回 null）；拣货从付款时开始，预约单从时段前 30 分钟开始
function markPaid(id, payBy) {
  const o = findOrder(id);
  if (!o || o.status !== 'unpaid') return null;
  if (payBy === 'wallet') {
    if (store.getWallet().balance < o.pay) return null;
    store.changeWallet(-o.pay, `订单支付 ${short(o.id)}`, 'pay');
  }
  const now = Date.now();
  const earn = Math.floor(o.pay);
  if (earn) store.changeBeans(earn, `消费得鲜豆 ${short(o.id)}`);
  return updateOrder(id, { status: 'paid', payBy, paidAt: now, earn, startAt: o.slotStart ? Math.max(now, o.slotStart - 30 * MIN) : now });
}

// 订单现在走到哪：stage 0 待付款 / 1 拣货中 / 2 配送中 / 3 已送达 / -1 已取消（已关闭）
function stageOf(o, now) {
  now = now || Date.now();
  const st = { stage: -1, steps: [], canPay: false, canCancel: false, canConfirm: false, canRate: false, canAfter: false, active: false, eta: '' };
  if (o.status === 'cancelled') {
    return Object.assign(st, { key: 'cancelled', status: o.paidAt ? '已取消' : '已关闭', tip: o.cancelReason || '订单已取消' });
  }
  if (o.status === 'unpaid') {
    const left = Math.max(0, o.payExpire - now);
    return Object.assign(st, { stage: 0, key: 'unpaid', status: '待付款', left, tip: `请在 ${mmss(left)} 内完成支付，超时订单自动关闭`, canPay: true, canCancel: true, active: true });
  }
  const t = now - o.startAt;
  let stage = 1;
  if (o.status === 'done' || t >= (ARRIVE_MIN + 3) * MIN) stage = 3;
  else if (t >= PICK_MIN * MIN) stage = 2;
  const arriveAt = o.slotEnd || o.startAt + ARRIVE_MIN * MIN;
  const tips = [
    '',
    t < 0 ? `预约订单，${store.hm(o.startAt)} 开始拣货，拣货前可随时取消` : '前置仓正在为你拣货打包，出库前可随时取消',
    `骑手${RIDER.name}已取货，预计 ${store.hm(arriveAt)} 前送达`,
    o.afterSales && o.afterSales.length ? '订单已送达，售后款项已原路退回' : '订单已送达，祝你用餐愉快'
  ];
  const deliveredAt = o.doneAt || o.startAt + ARRIVE_MIN * MIN;
  return Object.assign(st, {
    stage,
    key: ['', 'picking', 'delivering', 'done'][stage],
    status: STEPS[stage],
    tip: tips[stage],
    steps: STEPS.map((name, i) => ({ name, done: i <= stage })),
    canCancel: stage === 1,
    canConfirm: stage === 2,
    canRate: stage === 3 && !o.rating,
    canAfter: stage === 3 && now - deliveredAt < AFTER_DAYS * 24 * 60 * MIN && refundable(o).some(i => !i.refunded),
    active: stage < 3,
    eta: o.slotStart ? o.slotLabel : `预计 ${store.hm(arriveAt)} 前送达`
  });
}

function cancelOrder(id) {
  const o = findOrder(id);
  if (!o || !stageOf(o).canCancel) return null;
  const paid = o.status === 'paid';
  if (paid && o.payBy === 'wallet') store.changeWallet(o.pay, `订单退款 ${short(o.id)}`, 'refund');
  if (paid && o.earn) store.changeBeans(-o.earn, `订单取消扣回 ${short(o.id)}`);
  release(o);
  return updateOrder(id, {
    status: 'cancelled',
    cancelledAt: Date.now(),
    refund: paid ? o.pay : 0,
    cancelReason: paid ? `订单已取消，¥${o.pay} 已原路退回${o.payBy === 'wallet' ? '余额' : '微信'}` : '你取消了订单，优惠券与鲜豆已退回'
  });
}
function confirmOrder(id) {
  const o = findOrder(id);
  if (!o || !stageOf(o).canConfirm) return null;
  return updateOrder(id, { status: 'done', doneAt: Date.now() });
}

// 每件商品能退多少：按「实付里商品那部分 / 商品金额」摊；售后退过的不能再退
function refundable(o) {
  const feePaid = Math.max(0, (o.fee || 0) - (o.couponKind === 'ship' ? o.couponCut : 0));
  const ratio = o.goods ? Math.min(1, Math.max(0, (o.pay - feePaid) / o.goods)) : 0;
  const done = {};
  (o.afterSales || []).forEach(a => a.keys.forEach(k => { done[k] = true; }));
  return o.items.map(i => Object.assign({}, i, { refund: store.round2(i.total * ratio), refunded: !!done[i.key] }));
}
// 申请售后：坏单极速退款，提交即退
function applyAfterSale(id, keys, reason, note) {
  const o = findOrder(id);
  if (!o || !stageOf(o).canAfter) return null;
  const items = refundable(o).filter(i => keys.indexOf(i.key) >= 0 && !i.refunded);
  if (!items.length) return null;
  const amount = store.round2(items.reduce((n, i) => n + i.refund, 0));
  const rec = {
    id: 'AS' + Date.now().toString().slice(-8),
    keys: items.map(i => i.key),
    names: items.map(i => `${i.name}×${i.qty}`).join('、'),
    reason, note, amount,
    to: o.payBy === 'wallet' ? '余额' : '微信',
    time: store.dateTime(Date.now())
  };
  if (o.payBy === 'wallet') store.changeWallet(amount, `售后退款 ${short(o.id)}`, 'refund');
  updateOrder(id, { afterSales: (o.afterSales || []).concat([rec]), refund: store.round2((o.refund || 0) + amount) });
  return rec;
}

// 再来一单：按原规格加回购物车（下架的跳过，秒杀超限的只加到上限）
function reorder(o) {
  let lines = 0;
  let limited = false;
  o.items.forEach(i => {
    const gd = data.goods.find(x => x.id === i.id);
    if (!gd) return;
    const spec = gd.specs.some(s => s[0] === i.spec) ? i.spec : store.defaultSpec(gd);
    const r = store.addToCart(gd.id, i.qty, spec);
    if (r.ok) lines++;
    if (r.msg) limited = true;
  });
  return { lines, limited };
}

function rateOrder(id, rating) {
  const o = findOrder(id);
  if (!o || !stageOf(o).canRate) return null;
  updateOrder(id, { rating });
  store.changeBeans(20, `评价奖励 ${short(o.id)}`);
  return rating;
}
// 详情页「用户评价」里我自己的：评价过、含这件商品的订单
function myReviews(goodsId) {
  return getOrders().filter(o => o.rating && o.items.some(i => i.id === goodsId)).map(o => ({
    user: '我', stars: o.rating.stars, text: o.rating.text || o.rating.tags.join('、') || '默认好评', time: o.rating.time.slice(0, 10), mine: true
  }));
}

function match(tab, o, st) {
  if (tab === 'unpaid') return st.stage === 0;
  if (tab === 'receiving') return st.stage === 1 || st.stage === 2;
  if (tab === 'rate') return st.canRate;
  if (tab === 'after') return (o.afterSales || []).length > 0 || (st.stage === -1 && o.refund > 0);
  return true;
}
function counts() {
  const c = { unpaid: 0, receiving: 0, rate: 0, after: 0 };
  getOrders().forEach(o => {
    const st = stageOf(o);
    ['unpaid', 'receiving', 'rate'].forEach(k => { if (match(k, o, st)) c[k]++; });
  });
  return c;
}
// PLUS 本年已省：开卡以来 368.5 + 免运费券抵掉的运费
function plusSaved() {
  return store.round2(368.5 + getOrders().filter(o => o.paidAt && o.status !== 'cancelled' && o.couponKind === 'ship').reduce((n, o) => n + o.couponCut, 0));
}

module.exports = {
  TABS, RIDER, STEPS, getOrders, findOrder, updateOrder, create, markPaid, stageOf, cancelOrder, confirmOrder,
  refundable, applyAfterSale, reorder, rateOrder, myReviews, match, counts, plusSaved, short
};
