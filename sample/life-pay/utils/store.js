// 户号与缴费记录：存在本地 storage
const data = require('./data.js');

function pad(v) { return v < 10 ? '0' + v : '' + v; }
function stamp(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes()); }
function daysAgo(n, h, m) { const t = new Date(); return stamp(new Date(t.getFullYear(), t.getMonth(), t.getDate() - n, h, m)); }
function money(n) { return (Math.round(n * 100) / 100).toFixed(2); }

const DEFAULT_ACCOUNTS = [
  { id: 'a1', type: 'power', city: '杭州市', unit: '国网浙江省电力有限公司杭州供电公司', no: '3300881204', owner: '*明远', addr: '西湖区文三路 259 号 3 幢 1102', tag: '我家', owe: 213.65, period: '上月电费' },
  { id: 'a2', type: 'water', city: '杭州市', unit: '杭州市水务集团有限公司', no: '1180662701', owner: '*明远', addr: '西湖区文三路 259 号 3 幢 1102', tag: '我家', owe: 86.4, period: '近两月水费' },
  { id: 'a3', type: 'gas', city: '杭州市', unit: '杭州市燃气集团有限公司', no: '2290331815', owner: '*明远', addr: '西湖区文三路 259 号 3 幢 1102', tag: '我家', owe: 0, balance: 42.3, period: '' },
  { id: 'a4', type: 'net', city: '杭州市', unit: '中国电信杭州分公司', no: '057188231056', owner: '*明远', addr: '西湖区文三路 259 号 3 幢 1102', tag: '我家', owe: 0, balance: 0, period: '' },
  { id: 'a5', type: 'power', city: '杭州市', unit: '国网浙江省电力有限公司杭州供电公司', no: '3300764519', owner: '*建华', addr: '拱墅区湖墅南路 186 号 2 单元 501', tag: '父母家', owe: 98.2, period: '上月电费' }
];

const DEFAULT_RECORDS = [
  { id: 'r1', type: 'phone', title: '话费充值', target: '138****6621', amount: 100, paid: 99.5, time: daysAgo(3, 9, 12) },
  { id: 'r2', type: 'gas', title: '燃气费', target: '2290331815', amount: 120, paid: 120, time: daysAgo(9, 20, 41) },
  { id: 'r3', type: 'water', title: '水费', target: '1180662701', amount: 72.6, paid: 72.6, time: daysAgo(36, 19, 5) },
  { id: 'r4', type: 'power', title: '电费', target: '3300881204', amount: 186.3, paid: 186.3, time: daysAgo(38, 8, 30) },
  { id: 'r5', type: 'net', title: '宽带续费', target: '057188231056', amount: 199, paid: 199, time: daysAgo(44, 12, 16) },
  { id: 'r6', type: 'power', title: '电费', target: '3300764519', amount: 105.8, paid: 105.8, time: daysAgo(68, 10, 2) },
  { id: 'r7', type: 'phone', title: '话费充值', target: '139****2086', amount: 50, paid: 49.8, time: daysAgo(72, 21, 48) }
];

function getAccounts() {
  const v = wx.getStorageSync('lp_accounts');
  return Array.isArray(v) ? v : JSON.parse(JSON.stringify(DEFAULT_ACCOUNTS));
}
function saveAccounts(v) { wx.setStorageSync('lp_accounts', v); }
function getRecords() {
  const v = wx.getStorageSync('lp_records');
  return Array.isArray(v) ? v : JSON.parse(JSON.stringify(DEFAULT_RECORDS));
}
function saveRecords(v) { wx.setStorageSync('lp_records', v); }
function addRecord(r) {
  const list = getRecords();
  const rec = Object.assign({ id: 'r' + Date.now(), time: stamp(new Date()) }, r);
  list.unshift(rec);
  saveRecords(list);
  return rec;
}
// 户号展示：每 4 位空一格
function fmtNo(no) { return String(no).replace(/(\d{4})(?=\d)/g, '$1 '); }
function view(a) {
  const t = data.typeOf(a.type);
  return Object.assign({}, a, { tname: t.name, ch: t.ch, bg: t.bg, color: t.color, light: t.light, noTx: fmtNo(a.no), oweTx: money(a.owe), balTx: money(a.balance || 0) });
}

function getRecord(id) { return getRecords().find(r => String(r.id) === String(id)) || null; }
function updateRecord(id, patch) {
  const list = getRecords();
  const r = list.find(x => String(x.id) === String(id));
  if (r) Object.assign(r, patch);
  saveRecords(list);
  return r;
}
// 缴清一个户号：多缴的部分记进余额；返回缴费记录
function settle(accId, amt, extra) {
  const accs = getAccounts();
  const raw = accs.find(x => x.id === accId);
  const t = data.typeOf(raw.type);
  raw.balance = Math.round(((raw.balance || 0) + amt - raw.owe) * 100) / 100;
  raw.owe = 0;
  raw.period = '';
  saveAccounts(accs);
  const rec = addRecord(Object.assign({ type: raw.type, title: t.name, target: raw.no, amount: amt, paid: amt, unit: raw.unit, owner: raw.owner, accId: raw.id }, extra || {}));
  wx.setStorageSync('lp_last', rec);
  return rec;
}
// 自动缴费：{ cap: 单笔上限, ids: [开通的户号 id] }
function getAuto() {
  const v = wx.getStorageSync('lp_autopay');
  return v && typeof v === 'object' && Array.isArray(v.ids) ? v : { cap: 500, ids: [] };
}
function saveAuto(v) { wx.setStorageSync('lp_autopay', v); wx.setStorageSync('lp_auto', v.ids.length > 0); }
// 账单：按户号生成可复现的近 6 期账单（当期 = 欠费），用量 = 金额 / 单价
const PRICE = { water: { p: 4.2, u: '吨' }, power: { p: 0.538, u: '度' }, gas: { p: 3.0, u: '立方米' }, net: { p: 0, u: '' } };
function bills(a) {
  const seed = Number(String(a.no).slice(-4)) || 1;
  const now = new Date();
  const pr = PRICE[a.type] || PRICE.power;
  const list = [];
  for (let i = 0; i < 6; i++) {
    const d = new Date(now.getFullYear(), now.getMonth() - 1 - i, 1);
    let amt;
    if (i === 0 && a.owe > 0) amt = a.owe;
    else if (a.type === 'net') amt = 199;
    else amt = Math.round((60 + ((seed * (i + 3)) % 140)) * 100) / 100;
    const use = pr.p ? Math.round(amt / pr.p * 10) / 10 : 0;
    list.push({ ym: d.getFullYear() + '年' + (d.getMonth() + 1) + '月', amt, amtTx: money(amt), use, useTx: pr.p ? use + ' ' + pr.u : '包月套餐', price: pr.p ? pr.p + ' 元/' + pr.u : '', paid: !(i === 0 && a.owe > 0) });
  }
  return list;
}
module.exports = { getAccounts, saveAccounts, getRecords, saveRecords, addRecord, getRecord, updateRecord, settle, getAuto, saveAuto, bills, fmtNo, view, money, stamp };
