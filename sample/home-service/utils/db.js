// 本地存储：地址、订单、优惠券、发票。全部 wx.setStorageSync，退出重进还在。
const { SERVICES, STAFF, COUPONS } = require('./data.js');
const util = require('./util.js');

const K = { addr: 'hs_addr', orders: 'hs_orders', coupons: 'hs_coupons', invoices: 'hs_invoices', pick: 'hs_pick_addr', seeded: 'hs_seeded_v1' };

function seed() {
  if (wx.getStorageSync(K.seeded)) return;
  wx.setStorageSync(K.addr, [
    { id: 'ad1', name: '周女士', phone: '13816630271', region: ['上海市', '上海市', '徐汇区'], detail: '漕溪北路 595 号上海电影广场 3 号楼 1203 室', tag: '家', isDefault: true },
    { id: 'ad2', name: '周女士', phone: '13816630271', region: ['上海市', '上海市', '浦东新区'], detail: '张江路 665 号创新园 B 座 6 楼', tag: '公司', isDefault: false }
  ]);
  // 券：status 为 unused / used，过期按 expire 判断
  wx.setStorageSync(K.coupons, COUPONS.map((c) => Object.assign({}, c, { status: 'unused' })));
  const svc = SERVICES[2], staff = STAFF[1];
  wx.setStorageSync(K.orders, [{
    id: 'HG2026101914220', svcId: svc.id, svcName: svc.name, cover: svc.cover, specName: '挂式空调', qty: 2, unit: '台', price: 89,
    amount: 178, discount: 0, couponId: '', pay: 178, addr: { name: '周女士', phone: '13816630271', full: '上海市徐汇区漕溪北路 595 号上海电影广场 3 号楼 1203 室' },
    date: '2026-10-21', time: '09:30', remark: '卧室和客厅各一台，型号格力 KFR-35GW', photos: [], status: 'done',
    staffId: staff.id, createdAt: '2026-10-19 14:22', cancelReason: '', invoiced: false,
    review: { star: 5, tags: ['准时上门', '干活细致'], text: '拆下来的滤网脏得吓人，洗完出风明显干净了。', t: '2026-10-21 12:10' },
    timeline: [
      { t: '2026-10-19 14:22', text: '下单成功，已为你安排李春梅师傅' },
      { t: '2026-10-21 09:24', text: '师傅已上门，开始服务' },
      { t: '2026-10-21 11:05', text: '服务完成，已确认' },
      { t: '2026-10-21 12:10', text: '已评价 5 星' }
    ]
  }]);
  wx.setStorageSync(K.invoices, []);
  wx.setStorageSync(K.seeded, 1);
}

// ---------- 地址 ----------
function getAddrs() { return wx.getStorageSync(K.addr) || []; }
function getAddr(id) { return getAddrs().find((a) => a.id === id) || null; }
function fullAddr(a) { return a ? a.region.filter((x, i) => !(i === 1 && x === a.region[0])).join('') + a.detail : ''; }
function defaultAddr() { const l = getAddrs(); return l.find((a) => a.isDefault) || l[0] || null; }
function saveAddr(a) {
  const list = getAddrs();
  if (a.isDefault) list.forEach((x) => { x.isDefault = false; });
  if (!list.length) a.isDefault = true;
  const i = list.findIndex((x) => x.id === a.id);
  if (i >= 0) list[i] = a; else list.unshift(a);
  wx.setStorageSync(K.addr, list);
}
function removeAddr(id) {
  const list = getAddrs().filter((a) => a.id !== id);
  if (list.length && !list.some((a) => a.isDefault)) list[0].isDefault = true;
  wx.setStorageSync(K.addr, list);
}
function setDefaultAddr(id) { wx.setStorageSync(K.addr, getAddrs().map((a) => Object.assign(a, { isDefault: a.id === id }))); }
// 下单页「选地址」的结果：地址页 / 新增地址页写，下单页 onShow 读走
function setPicked(id) { wx.setStorageSync(K.pick, id); }
function takePicked() { const id = wx.getStorageSync(K.pick); if (id) wx.removeStorageSync(K.pick); return id || ''; }

// ---------- 订单 ----------
function getOrders() { return wx.getStorageSync(K.orders) || []; }
function getOrder(id) { return getOrders().find((o) => o.id === id) || null; }
function saveOrder(o) {
  const list = getOrders();
  const i = list.findIndex((x) => x.id === o.id);
  if (i >= 0) list[i] = o; else list.unshift(o);
  wx.setStorageSync(K.orders, list);
}
function removeOrder(id) { wx.setStorageSync(K.orders, getOrders().filter((o) => o.id !== id)); }
function pushTimeline(o, text) { o.timeline = (o.timeline || []).concat([{ t: util.stamp(), text }]); return o; }
function staffFor(svcId) { return STAFF.find((s) => s.skills.indexOf(svcId) >= 0) || STAFF[0]; }

// ---------- 优惠券 ----------
function couponState(c, today) {
  if (c.status === 'used') return 'used';
  return c.expire < today ? 'expired' : 'unused';
}
function getCoupons() {
  const today = util.ymd(new Date());
  return (wx.getStorageSync(K.coupons) || []).map((c) => Object.assign({}, c, { state: couponState(c, today) }));
}
function saveCoupons(list) { wx.setStorageSync(K.coupons, list.map((c) => { const o = Object.assign({}, c); delete o.state; return o; })); }
function usableCoupons(svcId, amount) {
  return getCoupons().filter((c) => c.state === 'unused' && (!c.cat || c.cat === svcId) && amount >= c.min);
}
function setCouponStatus(id, status) { saveCoupons(getCoupons().map((c) => (c.id === id ? Object.assign(c, { status }) : c))); }
function addCoupon(c) {
  const list = getCoupons();
  if (list.some((x) => x.id === c.id)) return false;
  list.unshift(Object.assign({}, c, { status: 'unused' }));
  saveCoupons(list);
  return true;
}

// ---------- 发票 ----------
function getInvoices() { return wx.getStorageSync(K.invoices) || []; }
function addInvoice(inv) {
  wx.setStorageSync(K.invoices, [inv].concat(getInvoices()));
  const ids = inv.orderIds;
  wx.setStorageSync(K.orders, getOrders().map((o) => (ids.indexOf(o.id) >= 0 ? Object.assign(o, { invoiced: true }) : o)));
}

module.exports = {
  seed, getAddrs, getAddr, fullAddr, defaultAddr, saveAddr, removeAddr, setDefaultAddr, setPicked, takePicked,
  getOrders, getOrder, saveOrder, removeOrder, pushTimeline, staffFor,
  getCoupons, usableCoupons, setCouponStatus, addCoupon, getInvoices, addInvoice
};
