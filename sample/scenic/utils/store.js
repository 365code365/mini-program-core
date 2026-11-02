// 本地数据：订单、常用游客、收藏、导览设置
const D = require('./date.js');
const data = require('./data.js');
const idc = require('./idcard.js');

const K = { orders: 'scenic_orders', tourists: 'scenic_tourists', favs: 'scenic_favs', set: 'scenic_settings' };
const STATUS = { paid: '待使用', used: '已使用', refunded: '已退款', expired: '已过期' };

function get(k, def) { const v = wx.getStorageSync(k); return v === '' || v == null ? def : v; }
function put(k, v) { wx.setStorageSync(k, v); }

function settings() { return Object.assign({ chime: true, autoNext: true }, get(K.set, {})); }
function saveSettings(p) { const s = Object.assign(settings(), p); put(K.set, s); return s; }

function tourists() { return get(K.tourists, []); }
function saveTourist(t) {
  const l = tourists(), i = l.findIndex((x) => x.id === t.id);
  if (i >= 0) l[i] = t; else l.push(t);
  put(K.tourists, l);
}
function removeTourist(id) { put(K.tourists, tourists().filter((x) => x.id !== id)); }

// 入园日已过的待使用订单：演示里按「已使用」处理（真实场景由闸机核销回写）
function orders() {
  const l = get(K.orders, []), t = D.today();
  let dirty = false;
  l.forEach((o) => { if (o.status === 'paid' && o.date < t) { o.status = 'used'; dirty = true; } });
  if (dirty) put(K.orders, l);
  return l.map((o) => Object.assign({}, o, { statusText: STATUS[o.status] }));
}
function order(no) { return orders().find((o) => o.no === no) || null; }
function newNo() {
  const d = new Date();
  return 'SH' + D.fmt(d).replace(/-/g, '') + D.pad(d.getHours()) + D.pad(d.getMinutes()) + String(d.getSeconds() * 1000 + d.getMilliseconds()).padStart(5, '0');
}
function createOrder(o) {
  const l = get(K.orders, []);
  const rec = Object.assign({ no: newNo(), status: 'paid', created: D.fmt() + ' ' + D.hm() }, o);
  // 入园码：订单号 + 证件后 4 位拼出来，电子票的二维码按它画
  rec.code = rec.no.slice(-8) + (rec.people[0] ? rec.people[0].idTail : '0000');
  l.unshift(rec);
  put(K.orders, l);
  return rec;
}
function setStatus(no, status) {
  const l = get(K.orders, []), o = l.find((x) => x.no === no);
  if (o) { o.status = status; if (status === 'refunded') o.refundAt = D.fmt() + ' ' + D.hm(); put(K.orders, l); }
}
function removeOrder(no) { put(K.orders, get(K.orders, []).filter((x) => x.no !== no)); }

function favs() { return get(K.favs, []); }
function isFav(id) { return favs().indexOf(id) >= 0; }
function toggleFav(id) {
  const f = favs(), i = f.indexOf(id);
  if (i >= 0) f.splice(i, 1); else f.unshift(id);
  put(K.favs, f);
  return i < 0;
}

// 首次打开：两位常用游客 + 一张上个月已经用过的故宫订单
function ensureSeed() {
  if (wx.getStorageSync(K.tourists) !== '') return;
  const a = idc.withCheck('34100219880612402'), b = idc.withCheck('34100219900923362');
  put(K.tourists, [
    { id: 't1', name: '林知远', idcard: a, phone: '13856021478', idTail: a.slice(-4) },
    { id: 't2', name: '许晴', idcard: b, phone: '13956108823', idTail: b.slice(-4) }
  ]);
  const s = data.find('gugong'), day = D.addDays(D.today(), -24);
  put(K.orders, [{
    no: 'SH' + day.replace(/-/g, '') + '093012488', status: 'used', created: D.addDays(day, -5) + ' 20:01',
    scenic: 'gugong', scenicName: s.name, cover: s.cover, date: day, items: [{ id: 'adult', name: '成人票', price: 60, n: 2 }],
    people: [{ name: '林知远', idMask: idc.mask(a), idTail: a.slice(-4) }, { name: '许晴', idMask: idc.mask(b), idTail: b.slice(-4) }],
    phone: '13856021478', total: 120, count: 2, code: '93012488' + a.slice(-4)
  }]);
  put(K.favs, ['jiuzhaigou']);
}

module.exports = {
  STATUS, settings, saveSettings, tourists, saveTourist, removeTourist, orders, order, createOrder, setStatus, removeOrder,
  favs, isFav, toggleFav, ensureSeed
};
