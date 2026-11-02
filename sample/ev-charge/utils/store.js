// 本地状态：钱包 / 充电会话 / 订单 / 车辆 / 充电券 / 收藏 / 发票，全部存在 storage 里，各页读同一份
const data = require('./data.js');
const K = {
  wallet: 'ev_wallet', orders: 'ev_orders', session: 'ev_session', fav: 'ev_fav', cars: 'ev_cars',
  coupons: 'ev_coupons', codes: 'ev_codes', invoices: 'ev_invoices', autopay: 'ev_autopay', recent: 'ev_recent'
};
const MIN_START = 10;      // 余额低于 10 元不能开始充电
const AUTO_LINE = 20;      // 免密自动充值：结算后余额低于 20 元
const AUTO_AMOUNT = 100;
function round2(n) { return Math.round(n * 100) / 100; }
function pad(n) { return n < 10 ? '0' + n : '' + n; }
function stamp(ts) {
  const d = new Date(ts || Date.now());
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}
function fullDate(ts) {
  const d = new Date(ts || Date.now());
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}
// getStorageSync 缺省给 ''：数组 / 对象按类型判，不写 `|| 默认值`
function list(key, seed) {
  const v = wx.getStorageSync(key);
  if (Array.isArray(v)) return v;
  return seed ? seed() : [];
}
// ───────── 钱包 ─────────
function getWallet() {
  const w = wx.getStorageSync(K.wallet);
  if (w && typeof w.balance === 'number') return w;
  return {
    balance: 86.4,
    logs: [
      { id: 'l1', title: '充电消费·国网文一西路站', time: '10-22 19:08', amount: -77.53, type: 'pay' },
      { id: 'l2', title: '微信支付充值', time: '10-20 12:31', amount: 100, type: 'recharge' },
      { id: 'l3', title: '充电消费·星联西溪银泰城站', time: '10-18 08:44', amount: -34.94, type: 'pay' }
    ]
  };
}
function balance() { return getWallet().balance; }
function changeWallet(amount, title, type) {
  const w = getWallet();
  w.balance = round2(w.balance + amount);
  w.logs.unshift({ id: 'l' + Date.now().toString(36) + w.logs.length, title, amount: round2(amount), type, time: stamp() });
  w.logs = w.logs.slice(0, 60);
  wx.setStorageSync(K.wallet, w);
  return w;
}
function autoPay() { return wx.getStorageSync(K.autopay) === true; }
function setAutoPay(on) { wx.setStorageSync(K.autopay, !!on); }
// ───────── 车辆 ─────────
function getCars() {
  return list(K.cars, () => [{ id: 'car1', plate: '浙A·D88520', model: '比亚迪 海豹', battery: 82.5, isDefault: true }]);
}
function findCar(id) { return getCars().find(c => c.id === id) || null; }
function defaultCar() { const l = getCars(); return l.find(c => c.isDefault) || l[0] || null; }
// 车牌统一成「浙A·D88520」：去空格、去点、转大写
const PLATE = /^[京津沪渝冀豫云辽黑湘皖鲁新苏浙赣鄂桂甘晋蒙陕吉闽贵粤青藏川宁琼][A-Z][A-Z0-9]{5,6}$/;
function normPlate(s) {
  const p = String(s || '').replace(/[\s·.\-]/g, '').toUpperCase();
  return PLATE.test(p) ? p.slice(0, 2) + '·' + p.slice(2) : '';
}
function saveCar(c) {
  let l = getCars();
  if (l.some(x => x.plate === c.plate && x.id !== c.id)) return '该车牌已添加';
  if (!c.id) c.id = 'car' + Date.now().toString(36);
  if (c.isDefault || !l.some(x => x.id !== c.id)) {
    l = l.map(x => Object.assign({}, x, { isDefault: false }));
    c.isDefault = true;
  }
  const i = l.findIndex(x => x.id === c.id);
  if (i >= 0) l[i] = c;
  else l.push(c);
  wx.setStorageSync(K.cars, l);
  return '';
}
function setDefaultCar(id) {
  wx.setStorageSync(K.cars, getCars().map(x => Object.assign({}, x, { isDefault: x.id === id })));
}
function removeCar(id) {
  const l = getCars().filter(x => x.id !== id);
  if (l.length && !l.some(x => x.isDefault)) l[0].isDefault = true;
  wx.setStorageSync(K.cars, l);
}
// ───────── 充电券 ─────────
function getCoupons() {
  return list(K.coupons, () => [
    { id: 'c1', title: '服务费 8 折券', kind: 'service', rate: 0.8, min: 0, expire: '2026-12-31', status: 'valid' },
    { id: 'c2', title: '满 50 减 5 元', kind: 'cash', amount: 5, min: 50, expire: '2026-12-31', status: 'valid' },
    { id: 'c0', title: '新用户 5 元券', kind: 'cash', amount: 5, min: 0, expire: '2026-11-30', status: 'used' }
  ]);
}
function validCoupons() { return getCoupons().filter(c => c.status === 'valid'); }
// 一张券在这一单上省多少：服务费折扣券只打服务费，满减券看总额
function couponCut(c, fee, service) {
  if (!c || fee <= 0) return 0;
  if (c.kind === 'service') return round2(service * (1 - c.rate));
  return fee >= c.min ? round2(Math.min(c.amount, fee)) : 0;
}
function bestCoupon(fee, service) {
  let best = null;
  let cut = 0;
  validCoupons().forEach(c => {
    const v = couponCut(c, fee, service);
    if (v > cut) { cut = v; best = c; }
  });
  return { coupon: best, cut };
}
function setCouponStatus(id, status) {
  wx.setStorageSync(K.coupons, getCoupons().map(c => (c.id === id ? Object.assign({}, c, { status }) : c)));
}
// 兑换码：返回 { coupon } 或 { err }
function redeem(code) {
  const k = String(code || '').trim().toUpperCase();
  const tpl = data.CODES[k];
  if (!tpl) return { err: '兑换码不存在' };
  const used = list(K.codes);
  if (used.indexOf(k) >= 0) return { err: '该兑换码已使用' };
  const c = Object.assign({ id: 'x' + Date.now().toString(36), status: 'valid', expire: fullDate(Date.now() + 30 * 86400000) }, tpl);
  const l = getCoupons();
  l.unshift(c);
  wx.setStorageSync(K.coupons, l);
  wx.setStorageSync(K.codes, used.concat([k]));
  return { coupon: c };
}
// ───────── 收藏 ─────────
function getFavs() { return list(K.fav); }
function toggleFav(id) {
  const l = getFavs();
  const on = l.indexOf(id) < 0;
  wx.setStorageSync(K.fav, on ? l.concat([id]) : l.filter(x => x !== id));
  return on;
}
// ───────── 电站（叠加当前会话：自己正在用的桩算「充电中」） ─────────
function getSession() { return wx.getStorageSync(K.session) || null; }
function stationView(s) {
  const sess = getSession();
  const terms = s.list.map(t => (sess && sess.no === t.no ? Object.assign({}, t, { status: 'busy', soc: sess.soc || sess.startSoc, mine: true }) : t));
  const fast = terms.filter(t => t.power > 7);
  const slow = terms.filter(t => t.power <= 7);
  const idle = l => l.filter(t => t.status === 'idle').length;
  return Object.assign({}, s, {
    list: terms,
    fastFree: idle(fast), fastTotal: fast.length, slowFree: idle(slow), slowTotal: slow.length
  });
}
function stations() { return data.stations.map(stationView); }
function station(id) { return stationView(data.getStation(id)); }
function terminal(no) {
  const hit = data.findTerminal(no);
  if (!hit) return null;
  const st = stationView(hit.station);
  return { station: st, terminal: st.list.find(t => t.no === hit.terminal.no) };
}
// ───────── 订单 ─────────
// 费用按「电费 + 服务费」分项，券抵扣后是实付
function priced(o) {
  const elecFee = round2(o.kwh * o.elec);
  const serviceFee = round2(o.kwh * o.service);
  const fee = round2(elecFee + serviceFee);
  return Object.assign(o, { elecFee, serviceFee, fee, pay: round2(fee - (o.cut || 0)) });
}
function getOrders() {
  return list(K.orders, () => data.ORDERS.map(o => priced(Object.assign({ status: 'paid', payBy: 'balance', invoiced: false }, o))));
}
function saveOrders(l) { wx.setStorageSync(K.orders, l); }
function findOrder(id) { return getOrders().find(o => o.id === String(id)) || null; }
function unpaidOrder() { return getOrders().find(o => o.status === 'unpaid') || null; }
function recent() { return list(K.recent, () => ['XX0128-02', 'GW2210-03', 'XJ3301-05']); }
// 开始充电前的检查：返回 '' 或错误；kind 告诉页面该引导去哪（充值 / 付款 / 加车）
function checkStart(no) {
  const hit = terminal(no);
  if (!hit) return { err: '未找到该终端，请核对桩身铭牌上的编号' };
  if (getSession()) return { err: '你有一笔充电正在进行', kind: 'session' };
  if (hit.terminal.status === 'busy') return { err: '该终端正在被使用' };
  if (hit.terminal.status === 'fault') return { err: '该终端维护中，请换一个' };
  const unpaid = unpaidOrder();
  if (unpaid) return { err: '有一笔充电订单待支付 ¥' + unpaid.pay.toFixed(2) + '，请先完成支付', kind: 'unpaid', id: unpaid.id };
  if (!defaultCar()) return { err: '请先添加车辆', kind: 'car' };
  if (balance() < MIN_START && !autoPay()) return { err: '账户余额低于 ' + MIN_START + ' 元，请先充值', kind: 'recharge' };
  return { err: '' };
}
// startSoc 按车牌算一个固定值：同一辆车每次演示从同一电量开始
function startSession(no) {
  const hit = terminal(no);
  const car = defaultCar();
  const seed = car.plate.split('').reduce((n, ch) => n + ch.charCodeAt(0), 0);
  const s = {
    no: hit.terminal.no, station: hit.station.id, power: hit.terminal.power,
    carId: car.id, plate: car.plate, battery: car.battery,
    startSoc: 20 + seed % 25, soc: 0, elapsed: 0, startAt: Date.now()
  };
  s.soc = s.startSoc;
  wx.setStorageSync(K.session, s);
  wx.setStorageSync(K.recent, [s.no].concat(recent().filter(x => x !== s.no)).slice(0, 4));
  return s;
}
function saveSession(s) { wx.setStorageSync(K.session, s); }
// 结束充电 → 结算：自动用最省的券；余额够就扣（开了免密就先补足），不够就留一笔待支付
function finishSession(kwh, soc) {
  const s = getSession();
  if (!s) return null;
  const p = data.currentPeriod();
  const st = data.getStation(s.station);
  const o = priced({
    id: 'EV' + fullDate(Date.now()).replace(/-/g, '') + stamp().slice(6).replace(':', '') + pad(new Date().getSeconds()),
    station: s.station, terminal: s.no, carId: s.carId, plate: s.plate,
    kwh: round2(kwh), minutes: Math.max(1, Math.round((s.elapsed || 0) / 60)),
    date: stamp(s.startAt), endAt: stamp(), period: p.name, elec: p.elec, service: p.service,
    startSoc: s.startSoc, endSoc: Math.floor(soc), couponTitle: '', cut: 0, invoiced: false
  });
  const best = bestCoupon(o.fee, o.serviceFee);
  if (best.coupon) {
    o.couponId = best.coupon.id;
    o.couponTitle = best.coupon.title;
    o.cut = best.cut;
    o.pay = round2(o.fee - best.cut);
    setCouponStatus(best.coupon.id, 'used');
  }
  let topped = 0;
  if (autoPay()) {
    while (balance() - o.pay < AUTO_LINE) {
      changeWallet(AUTO_AMOUNT, '免密自动充值', 'recharge');
      topped += AUTO_AMOUNT;
    }
  }
  if (balance() >= o.pay) {
    if (o.pay > 0) changeWallet(-o.pay, '充电消费·' + st.name.split('·').pop(), 'pay');
    o.status = 'paid';
    o.payBy = 'balance';
  } else {
    o.status = 'unpaid';
    o.payBy = '';
  }
  o.topped = topped;
  const l = getOrders();
  l.unshift(o);
  saveOrders(l);
  wx.removeStorageSync(K.session);
  return o;
}
function payOrder(id, by) {
  const l = getOrders();
  const o = l.find(x => x.id === id);
  if (!o || o.status !== 'unpaid') return null;
  if (by === 'balance') {
    if (balance() < o.pay) return null;
    changeWallet(-o.pay, '充电消费·' + data.getStation(o.station).name.split('·').pop(), 'pay');
  }
  o.status = 'paid';
  o.payBy = by;
  saveOrders(l);
  return o;
}
function stats() {
  const l = getOrders();
  const kwh = l.reduce((a, o) => a + o.kwh, 0);
  // 电动车比燃油车每度电约少排 0.6kg CO₂（演示估算）
  return { count: l.length, kwh: kwh.toFixed(1), co2: (kwh * 0.6).toFixed(1) };
}
// ───────── 发票 ─────────
function getInvoices() { return list(K.invoices); }
function invoiceable() { return getOrders().filter(o => o.status === 'paid' && !o.invoiced && o.pay > 0); }
function addInvoice(ids, info) {
  const can = invoiceable().filter(o => ids.indexOf(o.id) >= 0);
  if (!can.length) return null;
  const amount = round2(can.reduce((s, o) => s + o.pay, 0));
  const inv = Object.assign({ id: 'FP' + Date.now().toString().slice(-8), amount, orderIds: can.map(o => o.id), time: stamp() }, info);
  const l = getInvoices();
  l.unshift(inv);
  wx.setStorageSync(K.invoices, l);
  saveOrders(getOrders().map(o => (inv.orderIds.indexOf(o.id) >= 0 ? Object.assign({}, o, { invoiced: true, invoiceId: inv.id }) : o)));
  return inv;
}
module.exports = {
  MIN_START, AUTO_LINE, AUTO_AMOUNT, round2, pad, stamp,
  getWallet, balance, changeWallet, autoPay, setAutoPay,
  getCars, findCar, defaultCar, normPlate, saveCar, setDefaultCar, removeCar,
  getCoupons, validCoupons, couponCut, bestCoupon, redeem,
  getFavs, toggleFav,
  getSession, saveSession, stations, station, terminal, checkStart, startSession, finishSession,
  getOrders, findOrder, unpaidOrder, payOrder, recent, stats,
  getInvoices, invoiceable, addInvoice
};
