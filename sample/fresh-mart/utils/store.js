// 鲜到家超市的本地状态：购物车 / 优惠券 / 鲜豆 / 余额 / 地址 / 收藏 / PLUS 会员，全部存在 storage 里。
// 订单的生命周期在 orders.js。
const data = require('./data.js');
const K = {
  cart: 'fm_cart', checkout: 'fm_checkout',
  used: 'fm_used_coupons', extra: 'fm_extra_coupons', plusWeek: 'fm_plus_week', plus: 'fm_plus',
  beans: 'fm_beans', beansLog: 'fm_beans_log', sign: 'fm_sign',
  wallet: 'fm_wallet', addrs: 'fm_addrs', addrPick: 'fm_addr_pick', favs: 'fm_favs'
};
const MIN = 60000;
const DAY = 24 * 60 * MIN;
const FREE_LINE = 39;
const SHIP_FEE = 5;

const pad = data.pad;
const dateStr = data.dateStr;
function round2(n) {
  return Math.round(n * 100) / 100;
}
function hm(ts) {
  const d = new Date(ts);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}
function dateTime(ts) {
  return `${dateStr(ts)} ${hm(ts)}`;
}
function today() {
  return dateStr(Date.now());
}
// 'YYYY-MM-DD' → 当天 0 点的时间戳
function dayStart(s) {
  const p = s.split('-').map(Number);
  return new Date(p[0], p[1] - 1, p[2]).getTime();
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
function maskPhone(p) {
  return p && p.length === 11 ? p.slice(0, 3) + '****' + p.slice(7) : p;
}

// ───────── 购物车：[{ id, spec, qty, checked }]，同商品同规格合并 ─────────
function getCart() {
  return list(K.cart);
}
function saveCart(l) {
  wx.setStorageSync(K.cart, l);
}
function cartCount(l) {
  return (l || getCart()).reduce((n, x) => n + x.qty, 0);
}
function defaultSpec(gd) {
  return gd.specs && gd.specs[0] ? gd.specs[0][0] : gd.spec;
}
// 单价：规格加价 + 秒杀价（秒杀已抢光的按日常价）
function unitPrice(id, spec) {
  const gd = data.findGoods(id);
  const f = data.flashOf(id);
  const s = (gd.specs || []).find(x => x[0] === spec);
  return round2((f ? f.price : gd.price) + (s ? s[1] : 0));
}
// 秒杀商品这一单还能再加几件（同一商品各规格合计）；不是秒杀的不限
function flashRoom(id, l) {
  if (!data.flashOf(id)) return 99;
  const inCart = (l || getCart()).filter(x => x.id === id).reduce((n, x) => n + x.qty, 0);
  return Math.max(0, data.FLASH_LIMIT - inCart);
}
// 加购：{ ok, count, msg }。秒杀商品超过限购只加到上限，msg 说明原因
function addToCart(id, qty, spec) {
  const gd = data.findGoods(id);
  const sp = spec || defaultSpec(gd);
  const l = getCart();
  const want = qty || 1;
  const room = flashRoom(id, l);
  if (room <= 0) return { ok: false, count: cartCount(l), msg: `秒杀商品每单限购 ${data.FLASH_LIMIT} 件` };
  const n = Math.min(want, room);
  const hit = l.find(x => x.id === id && x.spec === sp);
  if (hit) {
    hit.qty = Math.min(99, hit.qty + n);
    hit.checked = true;
  } else {
    l.unshift({ id, spec: sp, qty: n, checked: true });
  }
  saveCart(l);
  return { ok: true, count: cartCount(l), msg: n < want ? `秒杀价每单限购 ${data.FLASH_LIMIT} 件，已加到上限` : '' };
}
// 购物车 / 结算单的一行展开成带名称、图片、单价的视图数据
function expand(x) {
  const gd = data.findGoods(x.id);
  const unit = unitPrice(x.id, x.spec);
  return {
    key: x.id + '|' + x.spec, id: x.id, cat: gd.cat, name: gd.name, img: gd.img, spec: x.spec, qty: x.qty,
    checked: x.checked !== false, unit, total: round2(unit * x.qty), flash: !!data.flashOf(x.id), origin: gd.origin
  };
}
function shipFee(goods) {
  return goods > 0 && goods < FREE_LINE ? SHIP_FEE : 0;
}
// 结算单：购物车勾选的，或详情页「立即购买」的那一件
function setCheckout(lines, from) {
  wx.setStorageSync(K.checkout, { from, lines: lines.map(x => ({ id: x.id, spec: x.spec, qty: x.qty })) });
}
function getCheckout() {
  const v = wx.getStorageSync(K.checkout);
  return v && Array.isArray(v.lines) && v.lines.length ? v : null;
}
function clearCheckout() {
  wx.removeStorageSync(K.checkout);
}

// ───────── PLUS 会员：每周 2 张免运费券 ─────────
function plusState() {
  let p = wx.getStorageSync(K.plus);
  if (!p || typeof p.expire !== 'string') {
    // 第一次打开：会员还有 23 天到期（记下来，之后不再顺延）
    p = { expire: dateStr(Date.now() + 23 * DAY) };
    wx.setStorageSync(K.plus, p);
  }
  const left = Math.round((dayStart(p.expire) - dayStart(today())) / DAY);
  return { active: left >= 0, expire: p.expire, left };
}
function renewPlus() {
  const p = plusState();
  const base = p.active ? dayStart(p.expire) : dayStart(today());
  const expire = dateStr(base + 365 * DAY);
  wx.setStorageSync(K.plus, { expire });
  grantShipCoupons('续费礼', dateStr(Date.now() + 30 * DAY));
  return expire;
}
function grantShipCoupons(tag, expire) {
  for (let i = 0; i < 2; i++) {
    grantCoupon({ kind: 'ship', amount: SHIP_FEE, min: 0, title: 'PLUS 免运费券', cond: `抵扣本单运费 · ${tag}`, expire }, 'ps');
  }
}
// 本周的 2 张：会员有效、这周还没发过才发，周日 24 点过期
function ensurePlusWeekly() {
  if (!plusState().active) return;
  const d = new Date();
  const monday = new Date(d.getFullYear(), d.getMonth(), d.getDate() - ((d.getDay() + 6) % 7)).getTime();
  const week = dateStr(monday);
  if (wx.getStorageSync(K.plusWeek) === week) return;
  wx.setStorageSync(K.plusWeek, week);
  grantShipCoupons('本周有效', dateStr(monday + 6 * DAY));
}

// ───────── 优惠券 ─────────
function allCoupons() {
  ensurePlusWeekly();
  const used = list(K.used);
  const now = today();
  return list(K.extra).concat(data.coupons).map(c => {
    let status = 'valid';
    if (used.indexOf(c.id) >= 0) status = 'used';
    else if (c.expired || c.expire < now) status = 'expired';
    return Object.assign({}, c, { status });
  });
}
function validCoupons() {
  return allCoupons().filter(c => c.status === 'valid');
}
// 一张券在这一单上能省多少（不满足条件 0）：lines 是要结算的商品行，fee 是运费
function couponCut(c, lines, fee) {
  if (!c) return 0;
  if (c.kind === 'ship') return fee;
  const base = round2(lines.filter(x => !c.cats || c.cats.indexOf(x.cat) >= 0).reduce((n, x) => n + x.total, 0));
  if (!base || base < c.min) return 0;
  return round2(Math.min(c.amount, base));
}
function bestCoupon(lines, fee) {
  let best = null;
  validCoupons().forEach(c => {
    if (couponCut(c, lines, fee) > couponCut(best, lines, fee)) best = c;
  });
  return best;
}
function useCoupon(id) {
  const used = list(K.used);
  if (used.indexOf(id) < 0) used.push(id);
  wx.setStorageSync(K.used, used);
}
function restoreCoupon(id) {
  wx.setStorageSync(K.used, list(K.used).filter(x => x !== id));
}
function grantCoupon(c, prefix) {
  const extra = list(K.extra);
  const coupon = Object.assign({ kind: 'cash', min: 0 }, c, {
    id: c.id || (prefix || 'x') + Date.now().toString(36) + extra.length,
    expire: c.expire || dateStr(Date.now() + 30 * DAY)
  });
  extra.unshift(coupon);
  wx.setStorageSync(K.extra, extra);
  return coupon;
}

// ───────── 鲜豆：签到 / 评价 / 消费得，结算 100 鲜豆抵 1 元 ─────────
const BEANS0 = 860;
function getBeans() {
  return num(K.beans, BEANS0);
}
function beansLog() {
  const v = wx.getStorageSync(K.beansLog);
  if (Array.isArray(v)) return v;
  return [
    { title: '历史订单累计', delta: 760, time: dateTime(Date.now() - 9 * DAY) },
    { title: '新人注册礼', delta: 100, time: dateTime(Date.now() - 40 * DAY) }
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
// 这一单最多能用多少鲜豆：整百、单笔最多 1000（10 元）、不超过应付的 20%
function beansUsable(amount) {
  return Math.max(0, Math.min(Math.floor(getBeans() / 100) * 100, 1000, Math.floor(amount * 0.2) * 100));
}
function signed() {
  return wx.getStorageSync(K.sign) === today();
}
function sign() {
  if (signed()) return null;
  wx.setStorageSync(K.sign, today());
  return changeBeans(20, '每日签到');
}

// ───────── 余额 ─────────
function getWallet() {
  const w = wx.getStorageSync(K.wallet);
  if (w && typeof w.balance === 'number') return w;
  return {
    balance: 42.6,
    logs: [
      { title: '坏单赔付 · 退回余额', amount: 12.6, type: 'refund', time: dateTime(Date.now() - 5 * DAY) },
      { title: '充值 ¥30', amount: 30, type: 'recharge', time: dateTime(Date.now() - 20 * DAY) }
    ]
  };
}
function changeWallet(amount, title, type) {
  const w = getWallet();
  w.balance = round2(w.balance + amount);
  w.logs.unshift({ title, amount: round2(amount), type, time: dateTime(Date.now()) });
  w.logs = w.logs.slice(0, 50);
  wx.setStorageSync(K.wallet, w);
  return w;
}

// ───────── 收货地址：前置仓只送北京市内 ─────────
function getAddrs() {
  const v = wx.getStorageSync(K.addrs);
  if (Array.isArray(v)) return v;
  return [
    { id: 'a1', name: '周鲜鲜', phone: '13912348826', region: ['北京市', '北京市', '朝阳区'], detail: '望京SOHO T3 座 1606', tag: '公司', isDefault: true },
    { id: 'a2', name: '周鲜鲜', phone: '13912348826', region: ['北京市', '北京市', '朝阳区'], detail: '花家地北里 8 号院 3 单元 502', tag: '家', isDefault: false },
    { id: 'a3', name: '周妈妈', phone: '13811235567', region: ['北京市', '北京市', '朝阳区'], detail: '方恒国际中心 A 座 1203', tag: '', isDefault: false }
  ];
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
function inRange(a) {
  return !!a && (a.region || [])[1] === '北京市';
}
// 列表 / 结算页用的展示字段
function addrView(a) {
  if (!a) return null;
  const r = a.region || [];
  const area = r[0] === r[1] ? r[0] + (r[2] || '') : r.join('');
  return Object.assign({}, a, { text: area + a.detail, phoneMask: maskPhone(a.phone), inRange: inRange(a) });
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

// ───────── 收藏 ─────────
function getFavs() {
  return list(K.favs).filter(id => data.goods.some(x => x.id === id));
}
function isFav(id) {
  return getFavs().indexOf(id) >= 0;
}
// 返回切换之后是否已收藏
function toggleFav(id) {
  const l = getFavs();
  const i = l.indexOf(id);
  if (i >= 0) l.splice(i, 1);
  else l.unshift(id);
  wx.setStorageSync(K.favs, l);
  return i < 0;
}

module.exports = {
  MIN, DAY, FREE_LINE, SHIP_FEE, round2, pad, hm, dateStr, dateTime, today, maskPhone,
  getCart, saveCart, cartCount, defaultSpec, unitPrice, flashRoom, addToCart, expand, shipFee,
  setCheckout, getCheckout, clearCheckout,
  plusState, renewPlus, allCoupons, validCoupons, couponCut, bestCoupon, useCoupon, restoreCoupon, grantCoupon,
  getBeans, beansLog, changeBeans, beansUsable, signed, sign, getWallet, changeWallet,
  getAddrs, upsertAddr, removeAddr, inRange, addrView, currentAddr, pickAddr,
  getFavs, isFav, toggleFav
};
