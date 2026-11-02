// 本机持久化：订单、组局、会员卡、联系人。键统一 sv_ 前缀。
const K = { orders: 'sv_orders', teams: 'sv_teams', card: 'sv_card', contact: 'sv_contact', draft: 'sv_draft' };
function get(k, def) { const v = wx.getStorageSync(k); return v === '' || v === undefined || v === null ? def : v; }
function set(k, v) { wx.setStorageSync(k, v); }

function orders() { return get(K.orders, []); }
function getOrder(id) { return orders().find((o) => o.id === id) || null; }
function upsertOrder(o) {
  const list = orders(); const i = list.findIndex((x) => x.id === o.id);
  if (i >= 0) list[i] = o; else list.unshift(o);
  set(K.orders, list); return o;
}
// 某场馆某天被本机订单占用的格子：{ '3-18': true }（场地号-小时）
function bookedCells(venueId, date) {
  const m = {};
  orders().forEach((o) => {
    if (o.venueId !== venueId || o.date !== date || o.status === 'cancelled') return;
    o.segments.forEach((s) => { for (let h = s.start; h < s.end; h++) m[s.court + '-' + h] = true; });
  });
  return m;
}

function card() { return get(K.card, { balance: 0, times: [], records: [] }); }
function saveCard(c) { set(K.card, c); }
function addRecord(c, title, amount) {
  c.records.unshift({ id: 'r' + Date.now() + Math.floor(Math.random() * 100), title, amount, at: Date.now() });
  if (c.records.length > 30) c.records.length = 30;
}

function teams() { return get(K.teams, null); }
function saveTeams(t) { set(K.teams, t); }

function contact() { return get(K.contact, { name: '', phone: '' }); }
function saveContact(c) { set(K.contact, c); }
function draft() { return get(K.draft, null); }
function setDraft(d) { set(K.draft, d); }

function clearAll() {
  const info = wx.getStorageInfoSync();
  (info.keys || []).forEach((k) => { if (k.indexOf('sv_') === 0) wx.removeStorageSync(k); });
}
module.exports = { orders, getOrder, upsertOrder, bookedCells, card, saveCard, addRecord, teams, saveTeams, contact, saveContact, draft, setDraft, clearAll };
