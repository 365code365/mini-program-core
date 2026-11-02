// 订单：创建、支付（先走 wx.requestPayment，失败退到页面里的模拟收银台）、出票、退票、改期
const D = require('./date');
const F = require('./flights');
const store = require('./store');

const STATUS = { unpaid: '待支付', paid: '已出票', changed: '已改期', refunded: '已退票', cancelled: '已取消' };
const PAY_MINUTES = 30;

function create(legs, pax, contact, ins, price) {
  const now = Date.now();
  const o = {
    id: String(now).slice(-10) + Math.floor(Math.random() * 90 + 10),
    status: 'unpaid', createdAt: now, expireAt: now + PAY_MINUTES * 60000,
    legs: JSON.parse(JSON.stringify(legs)), pax: pax.map(p => ({ id: p.id, name: p.name, idType: p.idType, idNo: p.idNo, age: p.age })),
    contact, ins, rows: price.rows, total: price.total, log: [{ t: now, text: '提交订单' }]
  };
  store.saveOrder(o);
  return o;
}

// 真支付：桌面端没有宿主支付能力会立刻 fail，交给 onFallback 打开模拟收银台
function requestPay(o, onPaid, onFallback) {
  wx.requestPayment({
    timeStamp: String(Math.floor(Date.now() / 1000)),
    nonceStr: 'yy' + o.id,
    package: 'prepay_id=wx' + o.id,
    signType: 'RSA',
    paySign: 'mock-sign-' + o.id,
    success: () => onPaid(markPaid(o.id, '微信支付')),
    fail: (err) => {
      if (err && /cancel/.test(err.errMsg || '')) return wx.showToast({ title: '已取消支付', icon: 'none' });
      onFallback(err);
    }
  });
}

function markPaid(id, method) {
  const o = store.order(id);
  if (!o || o.status !== 'unpaid') return o;
  o.status = 'paid';
  o.paidAt = Date.now();
  o.payMethod = method;
  // 电子客票号：结算代码 + 10 位
  o.pax.forEach((p, i) => { p.ticket = (880 + o.legs[0].flight.airline.charCodeAt(0) % 9) + '-' + String(o.paidAt + i * 7919).slice(-10); });
  o.log.push({ t: o.paidAt, text: method + '支付成功 ¥' + o.total });
  o.log.push({ t: o.paidAt + 60000, text: '出票成功' });
  store.saveOrder(o);
  return o;
}

function expired(o) { return o.status === 'unpaid' && Date.now() > o.expireAt; }

// 距起飞的小时数（按第一程）
function hoursBefore(o) {
  const f = o.legs[0].flight;
  const dep = D.parse(f.date).getTime() + f.dep * 60000;
  return (dep - Date.now()) / 3600000;
}

// 退票：每段每人按运价规则扣费，税费全退
function refundQuote(o) {
  const h = hoursBefore(o);
  let fee = 0;
  o.legs.forEach(l => { fee += F.fee(l.cabin.rule, l.cabin.price, 'refund', h) * o.pax.filter(p => p.age !== '婴儿').length; });
  return { fee, back: Math.max(0, o.total - fee), hours: h };
}

function refund(id, reason) {
  const o = store.order(id), q = refundQuote(o);
  o.status = 'refunded';
  o.refund = { fee: q.fee, back: q.back, reason, at: Date.now() };
  o.log.push({ t: Date.now(), text: '申请退票（' + reason + '），手续费 ¥' + q.fee + '，退款 ¥' + q.back + ' 原路退回' });
  store.saveOrder(o);
  return o;
}

// 改期：同航班改到别的日期，按规则收改期费（差价本示例按同价处理）
function changeQuote(o) {
  const h = hoursBefore(o), l = o.legs[0];
  return F.fee(l.cabin.rule, l.cabin.price, 'change', h) * o.pax.filter(p => p.age !== '婴儿').length;
}

function change(id, date) {
  const o = store.order(id), fee = changeQuote(o);
  const old = o.legs[0].flight.date;
  o.legs[0].flight.date = date;
  o.status = 'changed';
  o.changeFee = (o.changeFee || 0) + fee;
  o.log.push({ t: Date.now(), text: '改期 ' + D.md(old) + ' → ' + D.md(date) + '，改期费 ¥' + fee });
  store.saveOrder(o);
  return o;
}

function cancel(id) {
  const o = store.order(id);
  o.status = 'cancelled';
  o.log.push({ t: Date.now(), text: '取消订单' });
  store.saveOrder(o);
  return o;
}

// 行程文字（复制 / 加日历用）
function itinerary(o) {
  return o.legs.map((l, i) => {
    const f = l.flight;
    return (o.legs.length > 1 ? (i ? '返程 ' : '去程 ') : '') + D.md(f.date) + ' ' + f.airlineName + f.no + ' ' + f.fromName + f.depText + ' → ' + f.toName + f.arrText +
      '（' + f.depAirport + ' 出发）';
  }).join('\n') + '\n乘机人：' + o.pax.map(p => p.name).join('、') + '\n订单号：' + o.id;
}

module.exports = { STATUS, create, requestPay, markPaid, expired, refundQuote, refund, changeQuote, change, cancel, itinerary, hoursBefore };
