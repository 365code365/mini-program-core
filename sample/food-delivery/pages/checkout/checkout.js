const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const orders = require('../../utils/orders.js');
const host = require('../../utils/host.js');

const MIN = store.MIN;
const SLOT = 15 * MIN;
const TABLEWARE = ['按餐量提供', '不需要餐具（环保）', '1 份', '2 份', '3 份', '4 份以上'];
const REMARK_TAGS = ['少辣', '不要香菜', '多放米饭', '放门口，不用敲门', '到了打电话'];
const MAX_REMARK = 40;

// 送达时间：立即送出 + 45 分钟之后每 15 分钟一档（slots 是每档开始的时间戳，0 = 立即）
function timeOptions() {
  const now = Date.now();
  const labels = [`立即送出（约 ${store.hm(now + 30 * MIN)} 送达）`];
  const slots = [0];
  const start = Math.ceil((now + 45 * MIN) / SLOT) * SLOT;
  for (let i = 0; i < 6; i++) {
    slots.push(start + i * SLOT);
    labels.push(`${store.hm(start + i * SLOT)} - ${store.hm(start + (i + 1) * SLOT)}`);
  }
  return { slots, labels };
}
// 没手动挑过（user === undefined）就挑最省的一张；挑过就用挑的那张（'' = 不使用）
function pickCoupon(list, user, base) {
  if (user !== undefined) return list.find(c => c.id === user) || null;
  let best = null;
  list.forEach(c => { if (store.couponCut(c, base) > store.couponCut(best, base)) best = c; });
  return best;
}
function couponText(list, c, cut, user, base) {
  if (cut) return '-¥' + cut;
  if (c) return '当前订单不满足使用条件';
  if (user === '') return '不使用';
  const n = list.filter(x => store.couponCut(x, base) > 0).length;
  return n ? `${n} 个可用` : '暂无可用';
}

Page({
  data: {
    shop: {},
    items: [],
    count: 0,
    goods: 0,
    address: null,
    times: [],
    timeIdx: 0,
    remark: '',
    // 点快捷备注改了 remark 时 +1，让输入框重建（用户打过字的 input，setData 改 value 画面不跟）
    remarkSeq: 0,
    maxRemark: MAX_REMARK,
    remarkTags: [],
    tableware: TABLEWARE,
    tablewareIdx: 0,
    protect: true,
    packFee: 0,
    deliveryFee: 0,
    fullCut: 0,
    newCut: 0,
    shopCoupon: null,
    shopCut: 0,
    shopText: '',
    red: null,
    redCut: 0,
    redText: '',
    discount: 0,
    pay: 0,
    earn: 0,
    payBy: 'wechat',
    balance: 0
  },

  onLoad(options) {
    const c = store.getCart();
    const shop = data.findShop(options.shop || c.shopId || 's1');
    const t = timeOptions();
    this.slots = t.slots;
    this.setData({ shop, times: t.labels, protect: store.getFlag(store.K.protect) });
    this.syncTags('');
  },

  onShow() {
    // 红包页「选这张」写回来的（'none' = 不使用）
    const red = wx.getStorageSync(store.K.pickRed);
    if (red) {
      wx.removeStorageSync(store.K.pickRed);
      this.userRed = red === 'none' ? '' : red;
    }
    const sc = wx.getStorageSync(store.K.pickShop);
    if (sc) {
      wx.removeStorageSync(store.K.pickShop);
      this.userShop = sc === 'none' ? '' : sc;
    }
    const a = store.currentAddr();
    this.setData({
      address: a ? { tag: a.tag, name: a.name, gender: a.gender, phone: store.maskPhone(a.phone), text: store.addrText(a) } : null,
      balance: store.getWallet().balance
    });
    this.calc();
  },

  // 商品 → 店铺满减 / 新客立减 → 商家代金券 → 平台红包 → 打包费 + 配送费
  calc() {
    const shop = this.data.shop;
    const s = store.summary(shop.id);
    const packFee = store.round2(s.count * shop.packFee);
    const fullCut = orders.fullCut(shop, s.amount);
    const newCut = s.count ? orders.newCut(shop) : 0;
    const base1 = store.round2(s.amount - fullCut - newCut);
    const shopList = store.validCoupons('shop', shop.id);
    const shopCoupon = pickCoupon(shopList, this.userShop, base1);
    const shopCut = store.couponCut(shopCoupon, base1);
    const base2 = store.round2(base1 - shopCut);
    const redList = store.validCoupons('red');
    const red = pickCoupon(redList, this.userRed, base2);
    const redCut = store.couponCut(red, base2);
    const pay = s.count ? Math.max(0.01, store.round2(s.amount + packFee + shop.deliveryFee - fullCut - newCut - shopCut - redCut)) : 0;
    this.base1 = base1;
    this.base2 = base2;
    this.setData({
      items: s.list,
      count: s.count,
      goods: s.amount,
      packFee,
      deliveryFee: shop.deliveryFee,
      fullCut,
      newCut,
      shopCoupon: shopCut ? shopCoupon : null,
      shopCut,
      shopText: shopList.length ? couponText(shopList, shopCoupon, shopCut, this.userShop, base1) : '本店暂无',
      red: redCut ? red : null,
      redCut,
      redText: couponText(redList, red, redCut, this.userRed, base2),
      discount: store.round2(fullCut + newCut + shopCut + redCut),
      pay,
      earn: Math.floor(pay)
    });
  },

  onAddress() {
    wx.navigateTo({ url: '/pages/address/address?select=1' });
  },

  onTime(e) {
    this.setData({ timeIdx: Number(e.detail.value) });
  },

  syncTags(remark) {
    this.setData({ remarkTags: REMARK_TAGS.map(t => ({ t, on: remark.indexOf(t) >= 0 })) });
  },

  onRemark(e) {
    const v = (e.detail.value || '').slice(0, MAX_REMARK);
    this.setData({ remark: v });
    this.syncTags(v);
  },

  // 快捷备注：点一下追加，再点一下去掉
  onRemarkTag(e) {
    const t = e.currentTarget.dataset.t;
    let parts = this.data.remark.split('，').map(x => x.trim()).filter(Boolean);
    if (parts.indexOf(t) >= 0) parts = parts.filter(x => x !== t);
    else parts.push(t);
    const v = parts.join('，');
    if (v.length > MAX_REMARK) {
      wx.showToast({ title: `备注最多 ${MAX_REMARK} 字`, icon: 'none' });
      return;
    }
    this.setData({ remark: v, remarkSeq: this.data.remarkSeq + 1 });
    this.syncTags(v);
  },

  onTableware(e) {
    this.setData({ tablewareIdx: Number(e.detail.value) });
  },

  onProtect(e) {
    this.setData({ protect: e.detail.value });
  },

  onShopCoupon() {
    if (this.data.shopText === '本店暂无') {
      wx.showToast({ title: '本店暂无可用的代金券', icon: 'none' });
      return;
    }
    const cur = this.data.shopCoupon ? this.data.shopCoupon.id : '';
    wx.navigateTo({ url: `/pages/coupons/coupons?select=shop&shop=${this.data.shop.id}&base=${this.base1}&cur=${cur}` });
  },

  onRed() {
    const cur = this.data.red ? this.data.red.id : '';
    wx.navigateTo({ url: `/pages/coupons/coupons?select=red&base=${this.base2}&cur=${cur}` });
  },

  onPayBy(e) {
    const payBy = e.currentTarget.dataset.by;
    if (payBy === 'wallet' && this.data.balance < this.data.pay) {
      this.askRecharge();
      return;
    }
    this.setData({ payBy });
  },

  askRecharge() {
    wx.showModal({
      title: '余额不足',
      content: `可用余额 ¥${this.data.balance}，本单需支付 ¥${this.data.pay}`,
      confirmText: '去充值',
      success: r => { if (r.confirm) wx.navigateTo({ url: '/pages/wallet/wallet' }); }
    });
  },

  onBack() {
    if (getCurrentPages().length > 1) wx.navigateBack();
    else wx.redirectTo({ url: `/pages/shop/shop?id=${this.data.shop.id}` });
  },

  onSubmit() {
    if (this.paying) return;
    const d = this.data;
    if (!d.items.length) {
      wx.showToast({ title: '购物车是空的', icon: 'none' });
      return;
    }
    if (!d.address) {
      wx.showToast({ title: '请先添加收货地址', icon: 'none' });
      wx.navigateTo({ url: '/pages/address-edit/address-edit' });
      return;
    }
    if (d.goods < d.shop.minOrder) {
      wx.showToast({ title: `还差 ¥${store.round2(d.shop.minOrder - d.goods)} 起送`, icon: 'none' });
      return;
    }
    if (d.payBy === 'wallet' && d.balance < d.pay) {
      this.askRecharge();
      return;
    }
    const o = orders.createOrder({
      shopId: d.shop.id,
      items: d.items.map(i => ({ key: i.key, dishId: i.dishId, name: i.name, spec: i.spec, picks: i.picks, qty: i.qty, price: i.unit, total: i.total })),
      count: d.count,
      goods: d.goods,
      packFee: d.packFee,
      deliveryFee: d.deliveryFee,
      fullCut: d.fullCut,
      newCut: d.newCut,
      shopCouponId: d.shopCoupon ? d.shopCoupon.id : '',
      shopCouponTitle: d.shopCoupon ? d.shopCoupon.title : '',
      shopCut: d.shopCut,
      redId: d.red ? d.red.id : '',
      redTitle: d.red ? d.red.title : '',
      redCut: d.redCut,
      pay: d.pay,
      payBy: d.payBy,
      remark: d.remark.trim() || '无',
      tableware: TABLEWARE[d.tablewareIdx],
      protect: d.protect,
      slotAt: this.slots[d.timeIdx],
      timeText: d.timeIdx ? d.times[d.timeIdx] : '立即送出',
      address: { tag: d.address.tag, name: d.address.name + (d.address.gender || ''), phone: d.address.phone, detail: d.address.text }
    });
    store.clearCart();
    this.userRed = undefined;
    this.userShop = undefined;
    this.paying = true;
    this.payNow(o);
  },

  // 付款：微信走 host.pay（宿主没有支付能力时退回页内确认框），余额先确认；没付成就留在「待支付」
  payNow(o) {
    const payBy = this.data.payBy;
    const finish = ok => {
      this.paying = false;
      const url = '/pages/order-detail/order-detail?id=' + o.id;
      if (!ok) {
        wx.showToast({ title: '支付未完成，15 分钟内可继续支付', icon: 'none' });
        wx.redirectTo({ url });
        return;
      }
      const r = orders.payOrder(o.id, payBy);
      wx.showToast({ title: r.err || '支付成功', icon: r.err ? 'none' : 'success' });
      wx.redirectTo({ url });
    };
    if (payBy === 'wallet') {
      wx.showModal({
        title: '余额支付',
        content: `可用余额 ¥${this.data.balance}，本单支付 ¥${o.pay}`,
        confirmText: '确认支付',
        success: r => finish(!!r.confirm),
        fail: () => finish(false)
      });
      return;
    }
    host.pay(o.pay, `${this.data.shop.name} · ${o.count} 件商品`).then(finish);
  }
});
