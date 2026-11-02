// 跨页状态都落 storage：我的发布、收藏、会话、订单、搜索历史、曝光统计。退出重进还在
const D = require('./data.js');
const K = {
  seeded: 'xz_seeded', mine: 'xz_mine', fav: 'xz_fav', hidden: 'xz_hidden', reported: 'xz_reported', history: 'xz_history',
  expose: 'xz_expose', convs: 'xz_convs', orders: 'xz_orders', sold: 'xz_sold', comments: 'xz_comments', addr: 'xz_addr'
};
function get(k, def) { const v = wx.getStorageSync(k); return v === '' || v === undefined || v === null ? def : v; }
function set(k, v) { wx.setStorageSync(k, v); }
function pad(n) { return n < 10 ? '0' + n : '' + n; }
function today() { const d = new Date(); return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function timeText(ts) {
  const now = Date.now(), diff = now - ts, d = new Date(ts);
  if (diff < 60e3) return '刚刚';
  if (diff < 3600e3) return Math.floor(diff / 60e3) + ' 分钟前';
  if (new Date(now).toDateString() === d.toDateString()) return pad(d.getHours()) + ':' + pad(d.getMinutes());
  return (d.getMonth() + 1) + '月' + d.getDate() + '日';
}
function seed() {
  if (get(K.seeded, false)) return;
  const now = Date.now();
  set(K.mine, [{ id: 'm01', title: '考研英语黄皮书 2024 真题 + 单词书', price: 30, orig: 98, cond: '9成新', cat: 'book', imgs: ['/images/goods/g_books.jpg'],
    ratio: 1.0, desc: '一战上岸了，真题只做了 2015 年以后的，单词书有荧光笔划线。紫金港自取或邮费到付。', free: false, status: 'on',
    created: now - 3 * 86400e3, polished: '', wants: 3, views: 64 }]);
  set(K.convs, [
    { id: 'c1', peer: 's2', item: 'i09', unread: 2, updated: now - 25 * 60e3, msgs: [
      { from: 'me', type: 'text', text: '你好，耳机还在吗？', ts: now - 50 * 60e3 },
      { from: 'peer', type: 'text', text: '在的，去年双十一买的，发票还在', ts: now - 40 * 60e3 },
      { from: 'me', type: 'text', text: '耳罩有没有掉皮？', ts: now - 30 * 60e3 },
      { from: 'peer', type: 'text', text: '没有，一直放收纳包里，可以视频给你看', ts: now - 25 * 60e3 }] },
    { id: 'c2', peer: 's1', item: 'i01', unread: 1, updated: now - 3 * 3600e3, msgs: [
      { from: 'me', type: 'offer', price: 2400, state: 'declined', ts: now - 4 * 3600e3 },
      { from: 'peer', type: 'text', text: '最低 2600，诚心要的话可以包邮', ts: now - 3 * 3600e3 }] },
    { id: 'c3', peer: 's5', item: 'i08', unread: 0, updated: now - 2 * 86400e3, msgs: [
      { from: 'peer', type: 'text', text: '吹风机可以在凤起路地铁站交易哦', ts: now - 2 * 86400e3 - 600e3 },
      { from: 'me', type: 'text', text: '好的，我考虑一下', ts: now - 2 * 86400e3 }] }
  ]);
  set(K.orders, [{ id: 'XZ' + (now - 9 * 86400e3), item: snapshotOf(D.ITEMS[3]), price: 129, ship: 0, total: 129, way: 'express',
    addr: defaultAddrs()[0], remark: '', status: 'done', created: now - 9 * 86400e3, express: '中通 7830 1946 2251', rate: 5, review: '手环很新，卖家发货快' }]);
  set(K.sold, { i04: true });
  set(K.addr, defaultAddrs());
  set(K.seeded, true);
}
function defaultAddrs() {
  return [{ name: '陈一鸣', phone: '138****6620', region: '浙江省 杭州市 西湖区', detail: '文三路 478 号华星时代广场 A 座 1203' },
    { name: '陈一鸣', phone: '138****6620', region: '浙江省 杭州市 西湖区', detail: '浙江大学紫金港校区 蓝田 3 舍' }];
}
function snapshotOf(it) { return { id: it.id, title: it.title, cover: cover(it), price: it.price, cond: it.cond, seller: it.seller }; }
function cover(it) { return D.imgsOf(it)[0]; }
function decorate(it, mine) {
  const sold = get(K.sold, {});
  const s = mine ? D.ME : D.SELLERS[it.seller];
  return Object.assign({}, it, { imgs: D.imgsOf(it), cover: cover(it), sellerInfo: s, mine: !!mine, sold: !!sold[it.id] || it.status === 'sold',
    seller: mine ? 'me' : it.seller, postedText: it.created ? timeText(it.created) : (it.days === 0 ? '今天发布' : it.days + ' 天前发布') });
}
function mine() { seed(); return get(K.mine, []); }
function saveMine(list) { set(K.mine, list); }
// 首页 / 搜索用的全部在售宝贝：我上架中的 + 平台上的，排掉「不感兴趣」的
function allItems() {
  seed();
  const hidden = get(K.hidden, []);
  const my = mine().filter(m => m.status === 'on').map(m => decorate(m, true));
  return my.concat(D.ITEMS.filter(it => hidden.indexOf(it.id) < 0).map(it => decorate(it, false)));
}
function findItem(id) {
  const m = mine().find(x => x.id === id);
  if (m) return decorate(m, true);
  const it = D.ITEMS.find(x => x.id === id);
  return it ? decorate(it, false) : null;
}
function favs() { return get(K.fav, []); }
function isFav(id) { return favs().indexOf(id) >= 0; }
function toggleFav(id) {
  const f = favs(), i = f.indexOf(id);
  if (i >= 0) f.splice(i, 1); else f.unshift(id);
  set(K.fav, f);
  return i < 0;
}
function hide(id) { const h = get(K.hidden, []); if (h.indexOf(id) < 0) h.push(id); set(K.hidden, h); }
function report(id, reason) { const r = get(K.reported, []); r.push({ id, reason, ts: Date.now() }); set(K.reported, r); }
function addExpose(id) { const e = get(K.expose, {}); e[id] = (e[id] || 0) + 1; set(K.expose, e); return e[id]; }
function exposeOf(id) { return get(K.expose, {})[id] || 0; }
function exposeTotal() { const e = get(K.expose, {}); return Object.keys(e).reduce((s, k) => s + e[k], 0); }
// ---------- 会话 ----------
function convs() { seed(); return get(K.convs, []); }
function saveConvs(list) { set(K.convs, list); }
function unreadTotal() { return convs().reduce((s, c) => s + (c.unread || 0), 0); }
function convOf(id) { return convs().find(c => c.id === id); }
function convForItem(itemId) {
  const list = convs();
  let c = list.find(x => x.item === itemId);
  if (!c) {
    const it = findItem(itemId);
    c = { id: 'c' + Date.now(), peer: it.seller, item: itemId, unread: 0, updated: Date.now(), msgs: [] };
    list.unshift(c);
    saveConvs(list);
  }
  return c;
}
function pushMsg(convId, msg, unread) {
  const list = convs(), c = list.find(x => x.id === convId);
  if (!c) return null;
  msg.ts = msg.ts || Date.now();
  c.msgs.push(msg);
  c.updated = msg.ts;
  if (unread) c.unread = (c.unread || 0) + 1;
  saveConvs(list);
  return c;
}
function updateMsg(convId, index, patch) {
  const list = convs(), c = list.find(x => x.id === convId);
  if (c && c.msgs[index]) { Object.assign(c.msgs[index], patch); saveConvs(list); }
}
function markRead(convId) {
  const list = convs();
  list.forEach(c => { if (!convId || c.id === convId) c.unread = 0; });
  saveConvs(list);
}
function removeConv(convId) { saveConvs(convs().filter(c => c.id !== convId)); }
// ---------- 订单 ----------
function orders() { seed(); return get(K.orders, []); }
function orderOf(id) { return orders().find(o => o.id === id); }
function orderForItem(itemId) { return orders().find(o => o.item.id === itemId && o.status !== 'closed'); }
function createOrder(o) {
  const list = orders();
  list.unshift(o);
  set(K.orders, list);
  const sold = get(K.sold, {}); sold[o.item.id] = true; set(K.sold, sold);
}
function updateOrder(id, patch) {
  const list = orders(), o = list.find(x => x.id === id);
  if (o) { Object.assign(o, patch); set(K.orders, list); }
  return o;
}
// 订单状态变化同步到与卖家的会话里，聊天页顶部的交易卡读这个
function tradeNote(itemId, text) {
  const c = convForItem(itemId);
  pushMsg(c.id, { from: 'sys', type: 'status', text });
}
// ---------- 自定义 tabBar ----------
function syncTab(page, idx) {
  const tb = page.getTabBar && page.getTabBar();
  if (tb) tb.setData({ selected: idx, unread: unreadTotal() });
}
const STATUS = { paid: '待发货', shipped: '待收货', done: '已完成', closed: '已退款' };
module.exports = {
  K, get, set, today, timeText, seed, mine, saveMine, allItems, findItem, favs, isFav, toggleFav, hide, report,
  addExpose, exposeOf, exposeTotal, convs, saveConvs, unreadTotal, convOf, convForItem, pushMsg, updateMsg, markRead, removeConv,
  orders, orderOf, orderForItem, createOrder, updateOrder, tradeNote, syncTab, snapshotOf, STATUS, defaultAddrs
};
