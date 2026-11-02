// 本地状态：购物车 / 收货地址 / 红包卡券 / 余额 / 吃货豆 / 收藏 / 超级会员 / 设置，全部存在 storage 里。
// 订单与价格计算在 orders.js。
const data = require('./data.js');
const K = {
  cart: 'fd_cart', favs: 'fd_favs', notify: 'fd_notify', protect: 'fd_protect',
  addrs: 'fd_addrs', addrPick: 'fd_address',
  extra: 'fd_coupons', used: 'fd_used_coupons', redeemed: 'fd_redeemed',
  wallet: 'fd_wallet', beans: 'fd_beans', beansLog: 'fd_beans_log', vip: 'fd_vip',
  pickRed: 'fd_pick_red', pickShop: 'fd_pick_shop'
};
const MIN = 60000;
const DAY = 86400000;

function round2(n) {
  return Math.round(n * 100) / 100;
}
function pad(n) {
  return n < 10 ? '0' + n : '' + n;
}
function hm(ts) {
  const d = new Date(ts);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}
function dateStr(ts) {
  const d = new Date(ts);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}
function md(ts) {
  const d = new Date(ts);
  return `${d.getMonth() + 1}-${pad(d.getDate())}`;
}
function fmtTime(ts) {
  return `${md(ts)} ${hm(ts)}`;
}
function dateTime(ts) {
  return `${dateStr(ts)} ${hm(ts)}`;
}
// storage 里没有这一项时 getStorageSync 给 ''：数值不能用 `|| 默认值`（0 会被当成没有）
function num(key, def) {
  const v = wx.getStorageSync(key);
  return typeof v === 'number' ? v : def;
}
function list(key) {
  const v = wx.getStorageSync(key);
  return Array.isArray(v) ? v : [];
}

// ───────── 购物车（一单一店：换店点菜会清掉上一家）─────────
// 一行 = 一道菜的一种规格：{ key, dishId, spec, picks, unit, qty }，同菜同规格合并
function lineOf(shopId, dishId, picks, qty) {
  const d = data.findDish(shopId, dishId);
  if (!d) return null;
  const groups = data.specs[d.spec] || [];
  const names = [];
  let unit = d.price;
  const sel = groups.map((g, i) => {
    const k = Math.min(Math.max(0, (picks || [])[i] || 0), g.opts.length - 1);
    names.push(g.opts[k].n);
    unit += g.opts[k].add || 0;
    return k;
  });
  return { key: [dishId].concat(names).join('|'), dishId, spec: names.join(' / '), picks: sel, unit: round2(unit), qty: qty || 1 };
}
function getCart() {
  const c = wx.getStorageSync(K.cart);
  if (!c || !c.shopId) return { shopId: '', lines: [] };
  if (Array.isArray(c.lines)) return c;
  // 老版本存的是 { shopId, items: { dishId: qty } }
  const lines = Object.keys(c.items || {}).map(id => lineOf(c.shopId, id, [], c.items[id])).filter(Boolean);
  return { shopId: c.shopId, lines };
}
function saveCart(c) {
  wx.setStorageSync(K.cart, c.lines.length ? c : { shopId: '', lines: [] });
}
function clearCart() {
  saveCart({ shopId: '', lines: [] });
}
function addLine(shopId, line) {
  if (!line) return;
  let c = getCart();
  if (c.shopId !== shopId) c = { shopId, lines: [] };
  const hit = c.lines.find(x => x.key === line.key);
  if (hit) hit.qty = Math.min(99, hit.qty + line.qty);
  else c.lines.push(line);
  saveCart(c);
}
function changeLine(shopId, key, delta) {
  const c = getCart();
  if (c.shopId !== shopId) return;
  const i = c.lines.findIndex(x => x.key === key);
  if (i < 0) return;
  c.lines[i].qty = Math.min(99, c.lines[i].qty + delta);
  if (c.lines[i].qty <= 0) c.lines.splice(i, 1);
  saveCart(c);
}
// 不带规格的菜：key 就是菜的 id，列表上直接 + / −
function changeDish(shopId, dishId, delta) {
  if (delta > 0) addLine(shopId, lineOf(shopId, dishId, [], delta));
  else changeLine(shopId, dishId, delta);
}
function summary(shopId) {
  const c = getCart();
  const out = [];
  const counts = {};
  let count = 0;
  let amount = 0;
  (c.shopId === shopId ? c.lines : []).forEach(l => {
    const d = data.findDish(shopId, l.dishId);
    if (!d) return;
    out.push(Object.assign({}, l, { name: d.name, glyph: d.glyph, c1: d.c1, c2: d.c2, total: round2(l.unit * l.qty) }));
    counts[l.dishId] = (counts[l.dishId] || 0) + l.qty;
    count += l.qty;
    amount += l.unit * l.qty;
  });
  return { list: out, count, amount: round2(amount), counts };
}

// ───────── 红包卡券 ─────────
// kind：red 平台红包（全店通用）/ shop 商家代金券（只限 shopId 那家）
function baseCoupons(now) {
  return data.coupons.map(c => ({ id: c.id, kind: c.kind, shopId: c.shopId || '', title: c.title, amount: c.amount, min: c.min, expire: dateStr(now + c.days * DAY) }));
}
function allCoupons() {
  const now = Date.now();
  const today = dateStr(now);
  const used = list(K.used);
  return list(K.extra).concat(baseCoupons(now)).map(c => {
    const status = used.indexOf(c.id) >= 0 ? 'used' : c.expire < today ? 'expired' : 'valid';
    const shop = c.kind === 'shop' ? data.findShop(c.shopId) : null;
    return Object.assign({}, c, { status, shopName: shop ? shop.name : '全部商家可用' });
  });
}
function validCoupons(kind, shopId) {
  return allCoupons().filter(c => c.status === 'valid' && c.kind === kind && (kind !== 'shop' || c.shopId === shopId));
}
function couponCut(c, base) {
  if (!c || base < c.min) return 0;
  return round2(Math.min(c.amount, base));
}
function useCoupon(id) {
  const used = list(K.used);
  if (id && used.indexOf(id) < 0) used.push(id);
  wx.setStorageSync(K.used, used);
}
function restoreCoupon(id) {
  if (id) wx.setStorageSync(K.used, list(K.used).filter(x => x !== id));
}
// 发一张红包（会员、吃货豆兑换、兑换码），默认 30 天有效
function grantCoupon(c, prefix) {
  const extra = list(K.extra);
  const coupon = Object.assign({ kind: 'red', shopId: '', min: 0 }, c, {
    id: (prefix || 'x') + Date.now().toString(36) + extra.length,
    expire: c.expire || dateStr(Date.now() + 30 * DAY)
  });
  extra.unshift(coupon);
  wx.setStorageSync(K.extra, extra);
  return coupon;
}
// 兑换码：每个码只能用一次
const CODES = { BAOLE8: { title: '兑换码红包', amount: 8, min: 30 }, CHIHUO5: { title: '吃货节红包', amount: 5, min: 0 } };
function redeem(code) {
  const c = CODES[code];
  if (!c) return { err: '兑换码无效或已过期' };
  const done = list(K.redeemed);
  if (done.indexOf(code) >= 0) return { err: '这个兑换码已经用过了' };
  done.push(code);
  wx.setStorageSync(K.redeemed, done);
  return { coupon: grantCoupon(c, 'c') };
}

// ───────── 余额 ─────────
function getWallet() {
  const w = wx.getStorageSync(K.wallet);
  if (w && typeof w.balance === 'number') return w;
  return { balance: 88, logs: [{ title: '余额充值 ¥80 赠 ¥8', amount: 88, type: 'recharge', time: dateTime(Date.now() - 20 * DAY) }] };
}
function changeWallet(amount, title, type) {
  const w = getWallet();
  w.balance = round2(w.balance + amount);
  w.logs.unshift({ title, amount: round2(amount), type, time: dateTime(Date.now()) });
  w.logs = w.logs.slice(0, 50);
  wx.setStorageSync(K.wallet, w);
  return w;
}

// ───────── 吃货豆：消费 1 元得 1 个，评价得 20 个，可兑红包 ─────────
function getBeans() {
  return num(K.beans, 1260);
}
function beansLog() {
  const v = wx.getStorageSync(K.beansLog);
  if (Array.isArray(v)) return v;
  return [
    { title: '历史消费累计', delta: 1240, time: dateTime(Date.now() - 30 * DAY) },
    { title: '新人注册礼', delta: 20, time: dateTime(Date.now() - 90 * DAY) }
  ];
}
function changeBeans(delta, title) {
  const beans = Math.max(0, getBeans() + delta);
  wx.setStorageSync(K.beans, beans);
  const log = beansLog();
  log.unshift({ title, delta, time: dateTime(Date.now()) });
  wx.setStorageSync(K.beansLog, log.slice(0, 50));
  return beans;
}

// ───────── 超级会员：每开 / 续 1 个月发 4 张 ¥5 无门槛红包，有效期到会员到期日 ─────────
function getVip() {
  const v = wx.getStorageSync(K.vip);
  const until = v && typeof v.until === 'number' ? v.until : 0;
  return { active: until > Date.now(), until, untilText: until ? dateStr(until) : '', ever: !!until, price: until ? 10 : 1 };
}
function openVip() {
  const v = getVip();
  const until = Math.max(Date.now(), v.until) + 30 * DAY;
  wx.setStorageSync(K.vip, { until });
  for (let i = 0; i < 4; i++) grantCoupon({ title: '超级会员红包', amount: 5, min: 0, vip: true, expire: dateStr(until) }, 'v' + i);
  return getVip();
}

// ───────── 收货地址 ─────────
function getAddrs() {
  const v = wx.getStorageSync(K.addrs);
  if (Array.isArray(v)) return v;
  return data.addresses.map(a => Object.assign({}, a, { region: a.region.slice() }));
}
function saveAddrs(l) {
  wx.setStorageSync(K.addrs, l);
}
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
  saveAddrs(l);
  return a;
}
function removeAddr(id) {
  const l = getAddrs().filter(x => x.id !== id);
  if (l.length && !l.some(x => x.isDefault)) l[0].isDefault = true;
  saveAddrs(l);
  if (wx.getStorageSync(K.addrPick) === id) wx.removeStorageSync(K.addrPick);
}
function maskPhone(p) {
  return p && p.length === 11 ? p.slice(0, 3) + '****' + p.slice(7) : p;
}
function addrText(a) {
  return a ? `${a.detail} ${a.house || ''}`.trim() : '';
}
// 这一单送到哪：先看挑过的，再看默认地址
function currentAddr() {
  const l = getAddrs();
  const id = wx.getStorageSync(K.addrPick);
  return l.find(x => x.id === id) || l.find(x => x.isDefault) || l[0] || null;
}
function pickAddr(id) {
  wx.setStorageSync(K.addrPick, id);
}

// ───────── 收藏 / 设置 ─────────
function getFavs() {
  return list(K.favs);
}
function toggleFav(id) {
  const favs = getFavs();
  const i = favs.indexOf(id);
  if (i >= 0) favs.splice(i, 1);
  else favs.unshift(id);
  wx.setStorageSync(K.favs, favs);
  return i < 0;
}
// 没设置过（''）按开启算
function getFlag(key) {
  return wx.getStorageSync(key) !== false;
}
function setFlag(key, v) {
  wx.setStorageSync(key, !!v);
}

module.exports = {
  K, MIN, DAY, round2, pad, hm, dateStr, md, fmtTime, dateTime, num, list,
  lineOf, getCart, clearCart, addLine, changeLine, changeDish, summary,
  allCoupons, validCoupons, couponCut, useCoupon, restoreCoupon, grantCoupon, redeem,
  getWallet, changeWallet, getBeans, beansLog, changeBeans, getVip, openVip,
  getAddrs, upsertAddr, removeAddr, maskPhone, addrText, currentAddr, pickAddr,
  getFavs, toggleFav, getFlag, setFlag
};
