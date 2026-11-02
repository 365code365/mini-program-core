// 订单：提交 → 支付（15 分钟内）→ 商家接单 → 骑手到店 → 骑手取餐 → 送达，全部按时间推进；
// 取消 / 退款（钱、红包、吃货豆原路退回）、催单、确认收货、评价、再来一单。
const data = require('./data.js');
const store = require('./store.js');
const K = { orders: 'fd_orders' };
const MIN = store.MIN;
const PAY_LIMIT = 15 * MIN;
const URGE_GAP = 5 * MIN;
// 立即送出：付款后 1 分钟商家接单、9 分钟骑手到店、12 分钟取餐、30 分钟送达。
// 预约单把「送达」对齐到预约时段的开始，往前推 30 分钟开始备餐
const STEPS = [
  { name: '订单已提交', at: 0 },
  { name: '商家已接单', at: 1 },
  { name: '骑手已到店', at: 9 },
  { name: '骑手已取餐', at: 12 },
  { name: '已送达', at: 30 }
];
const PAY_NAME = { wechat: '微信支付', wallet: '余额支付' };

function mmss(ms) {
  const s = Math.ceil(ms / 1000);
  return store.pad(Math.floor(s / 60)) + ':' + store.pad(s % 60);
}

// ───────── 价格：满减 / 新客立减 ─────────
function fullCut(shop, amount) {
  let cut = 0;
  shop.tags.forEach(t => {
    const m = /^满(\d+)减(\d+)$/.exec(t);
    if (m && amount >= Number(m[1])) cut = Math.max(cut, Number(m[2]));
  });
  return cut;
}
function hasOrdered(shopId) {
  return getOrders().some(o => o.shopId === shopId && (o.status === 'paid' || o.status === 'done'));
}
// 「新客立减N」：在这家店还没有成功下过单
function newCut(shop) {
  const tag = shop.tags.map(t => /^新客立减(\d+)$/.exec(t)).find(Boolean);
  return tag && !hasOrdered(shop.id) ? Number(tag[1]) : 0;
}

// ───────── 历史订单示例（首次打开就有内容可看）─────────
function seedItem(shopId, dishId, qty, picks) {
  const l = store.lineOf(shopId, dishId, picks || [], qty);
  const d = data.findDish(shopId, dishId);
  return { key: l.key, dishId, name: d.name, spec: l.spec, picks: l.picks, qty, price: l.unit, total: store.round2(l.unit * qty) };
}
function seedOrder(id, shopId, items, ago, extra) {
  const shop = data.findShop(shopId);
  const goods = store.round2(items.reduce((n, i) => n + i.total, 0));
  const count = items.reduce((n, i) => n + i.qty, 0);
  const cut = fullCut(shop, goods);
  const packFee = count * shop.packFee;
  const createdAt = Date.now() - ago;
  const a = data.addresses[extra.addr || 0];
  const ex = Object.assign({}, extra);
  delete ex.addr;
  return Object.assign({
    id, shopId, status: 'done', createdAt, deadline: createdAt + PAY_LIMIT, paidAt: createdAt + 20000,
    startAt: createdAt + 20000, etaAt: createdAt + 20000 + 30 * MIN, doneAt: createdAt + 28 * MIN,
    items, count, goods, packFee, deliveryFee: shop.deliveryFee, fullCut: cut, newCut: 0,
    shopCouponId: '', shopCouponTitle: '', shopCut: 0, redId: '', redTitle: '', redCut: 0,
    pay: store.round2(goods + packFee + shop.deliveryFee - cut), payBy: 'wechat', earn: 0,
    remark: '无', tableware: '按餐量提供', protect: true, timeText: '立即送出',
    address: { tag: a.tag, name: a.name + a.gender, phone: store.maskPhone(a.phone), detail: store.addrText(a) }
  }, ex);
}
function seedOrders() {
  const DAY = store.DAY;
  return [
    seedOrder('BL' + (Date.now() - 14 * MIN).toString().slice(-10), 's1',
      [seedItem('s1', 'd101', 1), seedItem('s1', 'd102', 1), seedItem('s1', 'd108', 2)],
      14 * MIN, { status: 'paid', doneAt: 0, remark: '不要香菜', tableware: '1 份' }),
    seedOrder('BL' + (Date.now() - DAY).toString().slice(-10), 's4',
      [seedItem('s4', 'd401', 1, [0, 0]), seedItem('s4', 'd405', 2)], DAY, { remark: '少辣' }),
    seedOrder('BL' + (Date.now() - 3 * DAY).toString().slice(-10), 's5',
      [seedItem('s5', 'd501', 2, [0, 0, 0])], 3 * DAY,
      { addr: 1, rating: { stars: 5, tags: ['味道赞', '送餐快'], text: '杨枝甘露料很足，冰度刚好', time: store.dateTime(Date.now() - 3 * DAY + 40 * MIN) } }),
    seedOrder('BL' + (Date.now() - 6 * DAY).toString().slice(-10), 's3',
      [seedItem('s3', 'd301', 2)], 6 * DAY,
      { status: 'refund', doneAt: 0, refundReason: '商家出餐太慢', refundAt: Date.now() - 6 * DAY + 18 * MIN })
  ];
}

// ───────── 读写 ─────────
// 按当前时间推进：待支付超时 → 自动取消（红包退回）；配送中到了送达时间 → 已送达
function refresh(o, now) {
  if (o.status === 'unpaid' && now >= o.deadline) {
    o.status = 'cancelled';
    o.cancelReason = '超时未支付，订单已自动取消';
    o.cancelledAt = o.deadline;
    store.restoreCoupon(o.redId);
    store.restoreCoupon(o.shopCouponId);
    return true;
  }
  if (o.status === 'paid' && now >= o.etaAt) {
    o.status = 'done';
    o.doneAt = o.etaAt;
    return true;
  }
  return false;
}
function getOrders() {
  let l = wx.getStorageSync(K.orders);
  if (!Array.isArray(l)) {
    l = seedOrders();
    wx.setStorageSync(K.orders, l);
  }
  const now = Date.now();
  let changed = false;
  l.forEach(o => { if (refresh(o, now)) changed = true; });
  if (changed) wx.setStorageSync(K.orders, l);
  return l;
}
function findOrder(id) {
  return getOrders().find(o => o.id === id) || null;
}
function updateOrder(id, patch) {
  const l = getOrders();
  const o = l.find(x => x.id === id);
  if (!o) return null;
  Object.assign(o, patch);
  wx.setStorageSync(K.orders, l);
  return o;
}

// ───────── 状态 ─────────
function stageOf(o, now) {
  now = now || Date.now();
  const st = { key: o.status, stage: -1, canPay: false, canCancel: false, canRefund: false, canUrge: false, canConfirm: false, canRate: false, active: false, progress: 0 };
  if (o.status === 'unpaid') {
    const left = Math.max(0, o.deadline - now);
    return Object.assign(st, { text: '待支付', title: '等待支付', sub: `剩余 ${mmss(left)}，超时订单将自动取消`, left, canPay: true, canCancel: true });
  }
  if (o.status === 'cancelled') return Object.assign(st, { text: '已取消', title: '订单已取消', sub: o.cancelReason || '订单已取消' });
  if (o.status === 'refund') {
    const to = o.payBy === 'wallet' ? '余额' : '微信';
    return Object.assign(st, { text: '已退款', title: '退款成功', sub: `¥${o.pay} 已原路退回${to}，红包已退回账户` });
  }
  const done = o.status === 'done';
  let stage = 0;
  STEPS.forEach((s, i) => { if (done || now >= o.startAt + s.at * MIN) stage = i; });
  const eta = store.hm(o.etaAt);
  const titles = ['等待商家接单', '商家正在备餐', '骑手已到店', '骑手正在送餐', '订单已送达'];
  const subs = [
    o.startAt > now ? `预约订单，商家将于 ${store.hm(o.startAt)} 开始备餐` : '商家通常 1 分钟内接单，接单前可随时取消',
    `预计 ${eta} 送达 · 准时宝保障中`,
    `${data.RIDER.name}正在等商家出餐，预计 ${eta} 送达`,
    `预计 ${eta} 送达，骑手正在赶往你的位置`,
    '感谢使用饱了么，期待再次光临'
  ];
  // 骑手在路线上的位置：取餐之后按时间往终点挪
  let progress = [8, 20, 34, 62, 100][stage];
  if (stage === 3) {
    const pick = o.startAt + STEPS[3].at * MIN;
    progress = Math.round(62 + 38 * Math.min(1, Math.max(0, (now - pick) / (o.etaAt - pick))));
  }
  return Object.assign(st, {
    stage, text: done ? '已完成' : titles[stage], title: titles[stage], sub: subs[stage], progress, active: !done,
    canCancel: !done && stage === 0, canRefund: !done && stage >= 1, canUrge: !done && stage >= 1,
    canConfirm: !done && stage === 3, canRate: done && !o.rating
  });
}
// 时间线：配送单 5 步；取消 / 退款单 3 步
function stepsOf(o, now) {
  now = now || Date.now();
  const shop = data.findShop(o.shopId);
  const back = `¥${o.pay} 已退回${o.payBy === 'wallet' ? '余额' : '微信'}`;
  if (o.status === 'cancelled' || o.status === 'refund') {
    const at = o.status === 'refund' ? o.refundAt : (o.cancelledAt || o.createdAt);
    const paid = !!o.paidAt;
    return [
      { name: '订单已提交', desc: `${shop.name} · 共 ${o.count} 件`, time: store.hm(o.createdAt), done: true },
      { name: o.status === 'refund' ? '申请退款' : '订单已取消', desc: o.status === 'refund' ? '原因：' + o.refundReason : o.cancelReason, time: store.hm(at), done: true },
      { name: paid ? '退款成功' : '红包已退回', desc: paid ? back : '订单使用的红包已退回账户', time: store.hm(at), done: true, current: true }
    ];
  }
  const st = stageOf(o, now);
  const descs = [
    o.paidAt ? `已${PAY_NAME[o.payBy] || '支付'} ¥${o.pay}` : '等待支付',
    `${shop.name} 正在备餐`,
    `${data.RIDER.name}已到达商家，等待出餐`,
    '正在赶往你的位置',
    `送达 ${o.address.detail}`
  ];
  return STEPS.map((s, i) => {
    const ts = i === 0 ? o.createdAt : o.startAt + s.at * MIN;
    const done = o.status !== 'unpaid' && i <= st.stage;
    // 待支付：除了提交时间，其余时间点还没定
    let time = (done ? '' : '预计 ') + store.hm(i === 4 && o.doneAt ? o.doneAt : ts);
    if (o.status === 'unpaid') time = i ? '' : store.hm(ts);
    return { name: s.name, desc: descs[i], done, current: done && i === st.stage && st.active, time };
  });
}

// ───────── 下单 / 支付 ─────────
// 提交即锁定所用红包（取消、超时会退回）；15 分钟内未支付自动取消
function createOrder(info) {
  const now = Date.now();
  const o = Object.assign({
    id: 'BL' + now.toString().slice(-10), status: 'unpaid', createdAt: now, deadline: now + PAY_LIMIT,
    paidAt: 0, startAt: 0, etaAt: 0, doneAt: 0, earn: 0
  }, info);
  const l = getOrders();
  l.unshift(o);
  wx.setStorageSync(K.orders, l.slice(0, 40));
  store.useCoupon(o.redId);
  store.useCoupon(o.shopCouponId);
  return o;
}
// 返回 { order } 或 { err }
function payOrder(id, payBy) {
  const o = findOrder(id);
  if (!o || o.status !== 'unpaid') return { err: '订单已失效，请重新下单' };
  if (payBy === 'wallet') {
    const w = store.getWallet();
    if (w.balance < o.pay) return { err: '余额不足', short: true };
    store.changeWallet(-o.pay, `订单消费 ${data.findShop(o.shopId).name}`, 'pay');
  }
  const now = Date.now();
  const etaAt = o.slotAt ? Math.max(o.slotAt, now + 30 * MIN) : now + 30 * MIN;
  const earn = Math.floor(o.pay);
  if (earn) store.changeBeans(earn, '消费得吃货豆 ' + o.id);
  return { order: updateOrder(id, { status: 'paid', payBy, paidAt: now, startAt: etaAt - 30 * MIN, etaAt, earn }) };
}
// 钱、红包、吃货豆原路退回
function giveBack(o) {
  if (o.paidAt && o.payBy === 'wallet') store.changeWallet(o.pay, `订单退款 ${data.findShop(o.shopId).name}`, 'refund');
  store.restoreCoupon(o.redId);
  store.restoreCoupon(o.shopCouponId);
  if (o.earn) store.changeBeans(-o.earn, '订单退款扣回 ' + o.id);
}
// 待支付 / 商家接单前：直接取消
function cancelOrder(id) {
  const o = findOrder(id);
  if (!o || !stageOf(o).canCancel) return null;
  giveBack(o);
  const reason = o.paidAt ? `你取消了订单，¥${o.pay} 已原路退回` : '你取消了订单';
  return updateOrder(id, { status: 'cancelled', cancelReason: reason, cancelledAt: Date.now() });
}
// 商家接单后、送达前：申请退款（演示里商家秒同意）
function refundOrder(id, reason) {
  const o = findOrder(id);
  if (!o || !stageOf(o).canRefund) return null;
  giveBack(o);
  return updateOrder(id, { status: 'refund', refundReason: reason, refundAt: Date.now() });
}
function confirmOrder(id) {
  const o = findOrder(id);
  if (!o || !stageOf(o).canConfirm) return null;
  return updateOrder(id, { status: 'done', doneAt: Date.now() });
}
// 催单：5 分钟内只能催一次。返回 true 催到了
function urge(id) {
  const o = findOrder(id);
  if (!o || !stageOf(o).canUrge) return false;
  if (o.urgedAt && Date.now() - o.urgedAt < URGE_GAP) return false;
  updateOrder(id, { urgedAt: Date.now(), urgeCount: (o.urgeCount || 0) + 1 });
  return true;
}
function rateOrder(id, rating) {
  const o = findOrder(id);
  if (!o || !stageOf(o).canRate) return null;
  store.changeBeans(20, '评价奖励 ' + data.findShop(o.shopId).name);
  return updateOrder(id, { rating });
}
// 再来一单：按原规格放回购物车（换掉别家的购物车；下架的菜跳过）。返回放回的件数
function reorder(o) {
  store.clearCart();
  let n = 0;
  o.items.forEach(i => {
    const d = i.dishId ? data.findDish(o.shopId, i.dishId) : null;
    if (!d) return;
    store.addLine(o.shopId, store.lineOf(o.shopId, d.id, i.picks || [], i.qty));
    n += i.qty;
  });
  return n;
}

// ───────── 汇总（「我的」角标、本月已省）─────────
function counts() {
  const l = getOrders();
  return {
    unpaid: l.filter(o => o.status === 'unpaid').length,
    going: l.filter(o => o.status === 'paid').length,
    rate: l.filter(o => o.status === 'done' && !o.rating).length,
    refund: l.filter(o => o.status === 'refund').length,
    rated: l.filter(o => o.rating).length
  };
}
function savedThisMonth() {
  const d = new Date();
  const start = new Date(d.getFullYear(), d.getMonth(), 1).getTime();
  return store.round2(getOrders().filter(o => (o.status === 'paid' || o.status === 'done') && o.createdAt >= start)
    .reduce((n, o) => n + (o.fullCut || 0) + (o.newCut || 0) + (o.shopCut || 0) + (o.redCut || 0), 0));
}

module.exports = {
  K, PAY_LIMIT, PAY_NAME, mmss, fullCut, newCut, hasOrdered,
  getOrders, findOrder, updateOrder, stageOf, stepsOf, createOrder, payOrder, cancelOrder, refundOrder,
  confirmOrder, urge, rateOrder, reorder, counts, savedThisMonth
};
