// 萌宠家本地状态，全部存 storage：宠物档案（健康记录、体重）、订单（商城 / 服务预约）、收藏、地区、收货地址。
// 订单状态随时间推进（付款 2 分钟后发货、到了服务时间自动完成并更新宠物档案），页面 onShow 先 tick() 再读
const data = require('./data.js');
const K = { pets: 'pet_pets', current: 'pet_current', orders: 'pet_orders', favs: 'pet_favs', area: 'pet_area', addrs: 'pet_addrs', addrPick: 'pet_addr_pick' };
const DAY = 86400000;
const MIN = 60000;
const MAX_PETS = 5;
const CARD_TIMES = 3; // 洗护次卡：开卡送的 3 次基础洗护
const POINTS0 = 2680;
const PAY_LIMIT = 30 * MIN; // 待付款超过 30 分钟自动取消
const SHIP_AFTER = 2 * MIN; // 付款后 2 分钟发货（演示节奏）
const CANCEL_BEFORE = 2 * 3600 * 1000; // 服务开始前 2 小时内不能在线取消
const CYCLE = { 疫苗: 365, 驱虫: 90, 体检: 365 };

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function round2(n) { return Math.round(n * 100) / 100; }
function dateStr(ts) { const d = new Date(ts); return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function hm(ts) { const d = new Date(ts); return pad(d.getHours()) + ':' + pad(d.getMinutes()); }
function dateTime(ts) { return dateStr(ts) + ' ' + hm(ts); }
function today() { return dateStr(Date.now()); }
function parseDate(s) { const p = String(s).split('-').map(Number); return new Date(p[0], p[1] - 1, p[2]).getTime(); }
function addDays(s, n) { const p = String(s).split('-').map(Number); return dateStr(new Date(p[0], p[1] - 1, p[2] + n).getTime()); }
function daysBetween(a, b) { return Math.round((parseDate(b) - parseDate(a)) / DAY); }
// getStorageSync 没有这一项时给 ''：列表一律判 Array.isArray
function arr(key) { const v = wx.getStorageSync(key); return Array.isArray(v) ? v : null; }
let seq = 0;
function uid(p) { seq += 1; return p + Date.now().toString(36).toUpperCase() + seq; }

// ───────── 宠物档案 ─────────
function defaultPets() {
  const t = today();
  return data.pets.map(p => Object.assign({}, p, {
    records: p.records.map((r, i) => ({ id: p.id + 'r' + i, type: r.type, name: r.name, cycle: r.cycle, date: addDays(t, -r.ago), next: addDays(t, r.cycle - r.ago) })),
    weights: p.weights.map((v, i) => ({ date: addDays(t, -(p.weights.length - 1 - i) * 30), v }))
  }));
}
function getPets() { return arr(K.pets) || defaultPets(); }
function savePets(l) { wx.setStorageSync(K.pets, l); }
function findPet(id) { return getPets().find(p => p.id === id) || null; }
function currentPet() { const l = getPets(); return l.find(p => p.id === wx.getStorageSync(K.current)) || l[0] || null; }
function setCurrent(id) { wx.setStorageSync(K.current, id); }
function ageOf(birth) {
  if (!birth) return '年龄未知';
  const b = new Date(parseDate(birth));
  const n = new Date();
  let m = (n.getFullYear() - b.getFullYear()) * 12 + n.getMonth() - b.getMonth() - (n.getDate() < b.getDate() ? 1 : 0);
  if (m < 1) return '不满 1 个月';
  if (m < 12) return m + ' 个月';
  return Math.floor(m / 12) + ' 岁' + (m % 12 ? ' ' + (m % 12) + ' 个月' : '');
}
// 到期状态：过了下次日期 due，30 天内 soon
function recStatus(r) {
  const left = daysBetween(today(), r.next);
  return { status: left < 0 ? 'due' : (left <= 30 ? 'soon' : 'ok'), left, leftText: left < 0 ? '已过期 ' + -left + ' 天' : (left === 0 ? '今天到期' : left + ' 天后到期') };
}
function recordsOf(p) {
  const order = { due: 0, soon: 1, ok: 2 };
  return (p.records || []).map(r => Object.assign({}, r, recStatus(r))).sort((a, b) => order[a.status] - order[b.status] || (a.next < b.next ? -1 : 1));
}
function remindOf(p) {
  const rs = recordsOf(p);
  const due = rs.filter(r => r.status === 'due').length;
  const soon = rs.filter(r => r.status === 'soon').length;
  const r = rs[0];
  return {
    due, soon,
    text: r && r.status !== 'ok' ? r.leftText + '：' + r.name : (rs.length ? '健康档案已完善，各项提醒均正常' : '还没有健康记录，去档案里添加疫苗 / 驱虫')
  };
}
// 新增 / 编辑，返回错误文案（'' 成功）
function upsertPet(p) {
  const l = getPets();
  const i = l.findIndex(x => x.id === p.id);
  if (i < 0 && l.length >= MAX_PETS) return '最多添加 ' + MAX_PETS + ' 只宠物';
  if (l.some(x => x.name === p.name && x.id !== p.id)) return '已经有叫「' + p.name + '」的宠物了';
  if (i >= 0) {
    const old = l[i];
    l[i] = Object.assign({}, old, p, { records: old.records, weights: old.weights });
    if (p.weight !== old.weight) pushWeight(l[i], p.weight, today());
  } else {
    l.push(Object.assign({ records: [], weights: [{ date: today(), v: p.weight }] }, p));
  }
  savePets(l);
  return '';
}
function removePet(id) {
  if (getOrders().some(o => o.kind === 'service' && o.petId === id && o.status === 'booked')) return '还有待服务的预约，取消或完成后再删除';
  const l = getPets().filter(x => x.id !== id);
  savePets(l);
  if (wx.getStorageSync(K.current) === id) wx.setStorageSync(K.current, l.length ? l[0].id : '');
  return '';
}
function updatePet(id, fn) {
  const l = getPets();
  const p = l.find(x => x.id === id);
  if (!p) return null;
  fn(p);
  savePets(l);
  return p;
}
function addRecord(petId, r) {
  return updatePet(petId, p => {
    p.records = (p.records || []).concat({ id: uid('r'), type: r.type, name: r.name, cycle: r.cycle, date: r.date, next: addDays(r.date, r.cycle) });
  });
}
function removeRecord(petId, rid) { return updatePet(petId, p => { p.records = p.records.filter(r => r.id !== rid); }); }
// 这一项做过了：上次日期记为 date（默认今天），下次按周期顺延
function doneRecord(petId, rid, date) {
  return updatePet(petId, p => p.records.forEach(r => {
    if (r.id !== rid) return;
    r.date = date || today();
    r.next = addDays(r.date, r.cycle);
  }));
}
function pushWeight(p, v, date) {
  const ws = (p.weights || []).filter(w => w.date !== date).concat({ date, v });
  ws.sort((a, b) => (a.date < b.date ? -1 : 1));
  p.weights = ws.slice(-12);
  p.weight = p.weights[p.weights.length - 1].v;
}
function addWeight(petId, v, date) { return updatePet(petId, p => pushWeight(p, v, date || today())); }

// ───────── 订单：商城（goods）/ 服务预约（service） ─────────
// goods：unpaid 待付款 → paid 待发货 → shipped 待收货 → done（rating 为空即待评价）/ cancel
// service：booked 待服务 → done / cancel。下单即付款（微信或洗护次卡）
function getOrders() { return (arr(K.orders) || []).filter(o => o && o.kind); } // 老版本只存了 { id, count, total }，没有 kind 的丢掉
function saveOrders(l) { wx.setStorageSync(K.orders, l); }
function findOrder(id) { return getOrders().find(o => o.id === id) || null; }
function addOrder(o) { const l = getOrders(); l.unshift(o); saveOrders(l.slice(0, 50)); return o; }
function updateOrder(id, patch) {
  const l = getOrders();
  const o = l.find(x => x.id === id);
  if (!o) return null;
  Object.assign(o, patch);
  saveOrders(l);
  return o;
}
function createGoodsOrder(q) {
  return addOrder(Object.assign({ id: 'G' + Date.now().toString().slice(-10), kind: 'goods', status: 'unpaid', createdAt: Date.now() }, q));
}
function payOrder(id) { return updateOrder(id, { status: 'paid', paidAt: Date.now() }); }
function createServiceOrder(q) {
  const code = String(100000 + (Date.now() % 900000));
  return addOrder(Object.assign({ id: 'S' + Date.now().toString().slice(-10), kind: 'service', status: 'booked', code, createdAt: Date.now(), paidAt: Date.now() }, q));
}
function startOf(o) { const t = o.time.split(':').map(Number); return parseDate(o.date) + (t[0] * 60 + t[1]) * MIN; }
function endOf(o) { return startOf(o) + (o.cat === 'stay' ? (o.nights || 1) * DAY : Math.max(30, o.min || 0) * MIN); }
function canCancel(o, now) {
  now = now || Date.now();
  if (o.kind === 'goods') return o.status === 'unpaid' || o.status === 'paid';
  return o.status === 'booked' && now < startOf(o) - CANCEL_BEFORE;
}
// 取消：微信付的部分（pay）原路退回；用了洗护次卡的，次数由 cardLeft 按订单重算，自然退回
function cancelOrder(id) {
  const o = findOrder(id);
  if (!o || !canCancel(o)) return null;
  const paid = o.kind === 'service' || o.status === 'paid';
  return updateOrder(id, { status: 'cancel', cancelAt: Date.now(), refund: paid ? o.pay : 0, cardBack: !!o.cardUsed });
}
function confirmReceipt(id) {
  const o = findOrder(id);
  if (!o || o.status !== 'shipped') return null;
  return updateOrder(id, { status: 'done', doneAt: Date.now() });
}
function rateOrder(id, rating) { return updateOrder(id, { rating: Object.assign({ time: dateTime(Date.now()) }, rating) }); }
function stateOf(o, now) {
  now = now || Date.now();
  const base = { canCancel: canCancel(o, now), canPay: false, canConfirm: false, canRate: false };
  if (o.status === 'cancel') {
    const back = [o.refund ? '¥' + o.refund + ' 已原路退回' : '', o.cardBack ? '洗护卡 1 次已退回' : ''].filter(Boolean).join('，');
    return Object.assign(base, { text: '已取消', tip: back || (o.timeout ? '超时未支付，订单已关闭' : '订单已取消') });
  }
  if (o.kind === 'goods') {
    if (o.status === 'unpaid') return Object.assign(base, { text: '待付款', canPay: true, tip: '请在 ' + hm(o.createdAt + PAY_LIMIT) + ' 前完成支付，超时订单自动关闭' });
    if (o.status === 'paid') return Object.assign(base, { text: '待发货', tip: '仓库正在打包，付款后 2 分钟内发货' });
    if (o.status === 'shipped') return Object.assign(base, { text: '待收货', canConfirm: true, tip: '中通快递 ' + o.express + ' 运输中，预计明天送达' });
    return Object.assign(base, { text: o.rating ? '已完成' : '待评价', canRate: !o.rating, tip: '已签收，感谢支持萌宠家' });
  }
  if (o.status === 'booked') {
    return Object.assign(base, {
      text: '待服务',
      tip: base.canCancel ? '请于 ' + o.date.slice(5) + ' ' + o.time + ' 带' + o.petName + '到店，出示核销码 ' + o.code : '服务即将开始，如需改约请致电门店'
    });
  }
  return Object.assign(base, { text: o.rating ? '已完成' : '待评价', canRate: !o.rating, tip: '服务已完成，' + o.petName + '的档案已同步更新' });
}
// 服务做完：疫苗 / 驱虫 / 体检自动记进宠物档案（最该做的那一条顺延；没有就新建一条）
function applyService(o) {
  (o.svcs || []).forEach(s => {
    const svc = data.findService(s.id);
    if (!svc || !svc.record) return;
    updatePet(o.petId, p => {
      const rs = recordsOf(p).filter(r => r.type === svc.record);
      const hit = rs.length ? p.records.find(r => r.id === rs[0].id) : null;
      if (hit) {
        hit.date = o.date;
        hit.next = addDays(o.date, hit.cycle);
      } else {
        p.records = (p.records || []).concat({ id: uid('r'), type: svc.record, name: svc.name, cycle: CYCLE[svc.record], date: o.date, next: addDays(o.date, CYCLE[svc.record]) });
      }
    });
  });
}
function tick(now) {
  now = now || Date.now();
  const l = getOrders();
  const done = [];
  let changed = false;
  l.forEach(o => {
    if (o.kind === 'goods' && o.status === 'unpaid' && now - o.createdAt > PAY_LIMIT) {
      Object.assign(o, { status: 'cancel', cancelAt: o.createdAt + PAY_LIMIT, timeout: true });
      changed = true;
    } else if (o.kind === 'goods' && o.status === 'paid' && now - o.paidAt >= SHIP_AFTER) {
      Object.assign(o, { status: 'shipped', shipAt: o.paidAt + SHIP_AFTER, express: '7731' + String(o.paidAt).slice(-8) });
      changed = true;
    } else if (o.kind === 'service' && o.status === 'booked' && now >= endOf(o)) {
      Object.assign(o, { status: 'done', doneAt: endOf(o) });
      changed = true;
      done.push(o);
    }
  });
  if (changed) saveOrders(l);
  done.forEach(applyService);
}
// 会员资产都从订单推出来，取消 / 退款自然就回去了
function cardLeft() { return CARD_TIMES - getOrders().filter(o => o.kind === 'service' && o.cardUsed && o.status !== 'cancel').length; }
// 积分：实付 1 元 1 分（次卡抵扣的部分不算）
function points() {
  return POINTS0 + getOrders().filter(o => o.status !== 'cancel' && o.status !== 'unpaid').reduce((n, o) => n + Math.floor(o.pay || 0), 0);
}
// 新客首单：从没约过（没取消的）基础洗护
function isNewbie() { return !getOrders().some(o => o.kind === 'service' && o.status !== 'cancel' && (o.svcs || []).some(s => s.id === 's1')); }

// ───────── 收藏 / 地区 / 收货地址 ─────────
function getFavs() { return arr(K.favs) || []; }
function toggleFav(id) {
  const f = getFavs();
  const on = f.indexOf(id) < 0;
  wx.setStorageSync(K.favs, on ? f.concat(id) : f.filter(x => x !== id));
  return on;
}
function areaIdx() { const v = wx.getStorageSync(K.area); return typeof v === 'number' ? v : 0; }
function setArea(i) { wx.setStorageSync(K.area, i); }
function storesByArea() {
  const a = data.areas[areaIdx()];
  return data.stores.map(s => Object.assign({}, s, { km: a.dist[s.id], dist: a.dist[s.id] + 'km' })).sort((x, y) => x.km - y.km);
}
const ADDRS = [
  { id: 'a1', name: '林小满', phone: '18602312031', region: ['浙江省', '杭州市', '余杭区'], detail: '五常街道 西溪诚园 7 幢 2 单元 1102', tag: '家', isDefault: true },
  { id: 'a2', name: '林小满', phone: '18602312031', region: ['浙江省', '杭州市', '西湖区'], detail: '文三路 477 号 华星科技大厦 12F', tag: '公司', isDefault: false }
];
function getAddrs() { return arr(K.addrs) || ADDRS.map(a => Object.assign({}, a)); }
function upsertAddr(a) {
  let l = getAddrs();
  if (!a.id) a.id = 'a' + Date.now().toString(36);
  if (a.isDefault || !l.some(x => x.id !== a.id)) {
    l = l.map(x => Object.assign({}, x, { isDefault: false }));
    a.isDefault = true;
  }
  const i = l.findIndex(x => x.id === a.id);
  if (i >= 0) l[i] = a;
  else l.unshift(a);
  wx.setStorageSync(K.addrs, l);
  return a;
}
function removeAddr(id) {
  const l = getAddrs().filter(x => x.id !== id);
  if (l.length && !l.some(x => x.isDefault)) l[0].isDefault = true;
  wx.setStorageSync(K.addrs, l);
  if (wx.getStorageSync(K.addrPick) === id) wx.removeStorageSync(K.addrPick);
}
function maskPhone(p) { return p && p.length === 11 ? p.slice(0, 3) + '****' + p.slice(7) : p; }
function addrText(a) { return a.region.join('') + ' ' + a.detail; }
function currentAddr() {
  const l = getAddrs();
  return l.find(x => x.id === wx.getStorageSync(K.addrPick)) || l.find(x => x.isDefault) || l[0] || null;
}
function pickAddr(id) { wx.setStorageSync(K.addrPick, id); }

module.exports = {
  MAX_PETS, CARD_TIMES, MIN, DAY, pad, round2, dateStr, hm, dateTime, today, addDays, parseDate,
  getPets, findPet, currentPet, setCurrent, ageOf, recordsOf, remindOf, upsertPet, removePet, addRecord, removeRecord, doneRecord, addWeight,
  getOrders, findOrder, updateOrder, createGoodsOrder, payOrder, createServiceOrder, startOf, canCancel, cancelOrder, confirmReceipt, rateOrder, stateOf, tick,
  cardLeft, points, isNewbie,
  getFavs, toggleFav, areaIdx, setArea, storesByArea,
  getAddrs, upsertAddr, removeAddr, maskPhone, addrText, currentAddr, pickAddr
};
