// 本地持久化：搜索历史、乘机人、联系人、订单。退出重进都在。
const K = { history: 'fl_history', pax: 'fl_passengers', contact: 'fl_contact', orders: 'fl_orders', seeded: 'fl_seeded' };

function get(k, def) { const v = wx.getStorageSync(k); return v === '' || v === undefined || v === null ? def : v; }
function set(k, v) { wx.setStorageSync(k, v); }

// 首次打开给账号本人一条乘机人（真实格式、校验位正确的示例证件号）
function seed() {
  if (get(K.seeded, false)) return;
  set(K.pax, [{ id: 'p1', name: '林晓舟', idType: 0, idNo: '330106199203151230', phone: '13857162046', self: true }]);
  set(K.contact, { name: '林晓舟', phone: '13857162046' });
  set(K.seeded, true);
}

function history() { return get(K.history, []); }
function addHistory(h) {
  const key = h.from + '-' + h.to + '-' + h.trip;
  const list = history().filter(x => x.from + '-' + x.to + '-' + x.trip !== key);
  list.unshift(h);
  set(K.history, list.slice(0, 6));
}
function clearHistory() { set(K.history, []); }

function passengers() { return get(K.pax, []); }
function savePassenger(p) {
  const list = passengers();
  const i = list.findIndex(x => x.id === p.id);
  if (i >= 0) list[i] = p; else list.push(Object.assign({ id: 'p' + Date.now() }, p));
  set(K.pax, list);
}
function removePassenger(id) { set(K.pax, passengers().filter(p => p.id !== id)); }

function contact() { return get(K.contact, { name: '', phone: '' }); }
function saveContact(c) { set(K.contact, c); }

function orders() { return get(K.orders, []); }
function order(id) { return orders().find(o => o.id === id) || null; }
function saveOrder(o) {
  const list = orders();
  const i = list.findIndex(x => x.id === o.id);
  if (i >= 0) list[i] = o; else list.unshift(o);
  set(K.orders, list);
}
function removeOrder(id) { set(K.orders, orders().filter(o => o.id !== id)); }

module.exports = { seed, history, addHistory, clearHistory, passengers, savePassenger, removePassenger,
  contact, saveContact, orders, order, saveOrder, removeOrder };
