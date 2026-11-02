// 本机持久化：订单、想看、开售提醒、观演人、下单草稿。键统一 ls_ 前缀，清缓存时按前缀删。
const K = { orders: 'ls_orders', want: 'ls_want', remind: 'ls_remind', viewers: 'ls_viewers', draft: 'ls_draft', pick: 'ls_pick' };

function get(k, def) { const v = wx.getStorageSync(k); return v === '' || v === undefined || v === null ? def : v; }
function set(k, v) { wx.setStorageSync(k, v); }

function orders() { return get(K.orders, []); }
function saveOrders(list) { set(K.orders, list); }
function getOrder(id) { return orders().find((o) => o.id === id) || null; }
function upsertOrder(o) {
  const list = orders();
  const i = list.findIndex((x) => x.id === o.id);
  if (i >= 0) list[i] = o; else list.unshift(o);
  saveOrders(list);
  return o;
}
// 某场次被本机订单占用的座位（待支付锁座 + 已支付）
function takenSeats(sessionId) {
  const m = {};
  orders().forEach((o) => {
    if (o.sessionId !== sessionId || (o.status !== 'paid' && o.status !== 'unpaid')) return;
    o.items.forEach((it) => { if (it.seatId) m[it.seatId] = true; });
  });
  return m;
}

function wants() { return get(K.want, []); }
function isWant(id) { return wants().indexOf(id) >= 0; }
function toggleWant(id) {
  const list = wants();
  const i = list.indexOf(id);
  if (i >= 0) list.splice(i, 1); else list.unshift(id);
  set(K.want, list);
  return i < 0;
}

function reminds() { return get(K.remind, {}); }
function setRemind(id, how) { const m = reminds(); m[id] = { how, at: Date.now() }; set(K.remind, m); }
function clearRemind(id) { const m = reminds(); delete m[id]; set(K.remind, m); }

function viewers() { return get(K.viewers, []); }
function saveViewers(list) { set(K.viewers, list); }

function draft() { return get(K.draft, null); }
function setDraft(d) { set(K.draft, d); }

function pick() { return get(K.pick, []); }
function setPick(ids) { set(K.pick, ids); }

function clearAll() {
  const info = wx.getStorageInfoSync();
  (info.keys || []).forEach((k) => { if (k.indexOf('ls_') === 0) wx.removeStorageSync(k); });
}

module.exports = {
  K, orders, getOrder, upsertOrder, takenSeats,
  wants, isWant, toggleWant, reminds, setRemind, clearRemind,
  viewers, saveViewers, draft, setDraft, pick, setPick, clearAll
};
