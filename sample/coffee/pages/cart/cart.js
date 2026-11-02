const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const SLOT = 15 * store.MIN;
// 取餐时间：立即制作 + 20 分钟之后每 15 分钟一档（slots 是每档的时间戳，和文字一一对应）
function pickTimes() {
  const start = Math.ceil((Date.now() + 20 * store.MIN) / SLOT) * SLOT;
  const slots = [0];
  for (let i = 0; i < 6; i++) slots.push(start + i * SLOT);
  return { slots, labels: slots.map((t, i) => (i ? '预约 ' + store.hm(t) : '立即制作（约 5 分钟）')) };
}
function addrText(a) {
  if (!a) return '';
  const r = a.region || [];
  return (r[0] === r[1] ? r[0] + (r[2] || '') : r.join('')) + a.detail;
}
Page({
  data: {
    mode: 'pickup',
    store: data.stores[0],
    addr: null,
    pickTimes: [],
    pickIdx: 0,
    list: [],
    goods: 0,
    activityCut: 0,
    memberCut: 0,
    coupon: null,
    couponCut: 0,
    couponText: '',
    deliveryFee: 0,
    usePoints: false,
    usePointsNum: 0,
    pointsCut: 0,
    remark: '',
    payBy: 'wechat',
    balance: 0,
    pay: 0,
    saved: 0,
    earn: 0
  },
  onLoad() {
    const t = pickTimes();
    this.slots = t.slots;
    this.setData({ pickTimes: t.labels });
  },
  onShow() {
    // 优惠券页「去使用」会把选中的券写在这里
    const picked = wx.getStorageSync('cf_pick_coupon');
    if (picked) {
      wx.removeStorageSync('cf_pick_coupon');
      this.userCoupon = picked;
    }
    const a = store.currentAddr();
    this.setData({
      mode: store.getMode(),
      store: data.findStore(getApp().globalData.storeId),
      addr: a ? Object.assign({}, a, { text: addrText(a), phoneMask: store.maskPhone(a.phone) }) : null
    });
    this.calc();
  },
  // 价格：饮品 → 活动（新品第 2 杯半价）→ 会员日 8 折 → 优惠券 → 配送费 → 积分抵扣
  calc() {
    const s = store.summary();
    const mode = this.data.mode;
    const activityCut = store.activityCut(s.list);
    const memberCut = store.memberDay(store.levelOf(store.cups())) ? store.round2((s.amount - activityCut) * 0.2) : 0;
    const base = store.round2(s.amount - activityCut - memberCut);
    const coupons = store.validCoupons();
    const cut = c => store.couponCut(c, base, s.list, mode);
    let coupon = null;
    if (this.userCoupon !== undefined) {
      coupon = coupons.find(c => c.id === this.userCoupon) || null;
    } else {
      // 默认挑最省的一张
      coupons.forEach(c => { if (cut(c) > cut(coupon)) coupon = c; });
    }
    const couponCut = cut(coupon);
    const usable = coupons.filter(c => cut(c) > 0).length;
    let couponText = usable ? `${usable} 张可用` : '暂无可用';
    if (couponCut) couponText = '-¥' + couponCut;
    else if (coupon) couponText = '当前订单不满足使用条件';
    else if (this.userCoupon === '') couponText = '不使用优惠券';
    const deliveryFee = mode === 'delivery' && s.amount < 29 ? 6 : 0;
    const beforePoints = store.round2(base - couponCut + deliveryFee);
    // 100 积分抵 1 元，单笔最多 500 积分，且不能把实付抵到 0
    const usePointsNum = Math.min(500, Math.floor(store.getPoints() / 100) * 100, Math.max(0, Math.floor(beforePoints - 0.01)) * 100);
    const pointsCut = usePointsNum / 100;
    const usePoints = this.data.usePoints && usePointsNum > 0;
    const pay = Math.max(0.01, store.round2(beforePoints - (usePoints ? pointsCut : 0)));
    const balance = store.getWallet().balance;
    this.setData({
      list: s.list,
      goods: s.amount,
      activityCut,
      memberCut,
      coupon: couponCut ? coupon : null,
      couponCut,
      couponText,
      deliveryFee,
      usePoints,
      usePointsNum,
      pointsCut,
      pay,
      balance,
      saved: store.round2(activityCut + memberCut + couponCut + (usePoints ? pointsCut : 0)),
      earn: Math.floor(pay)
    });
  },
  onMode(e) {
    store.setMode(e.currentTarget.dataset.mode);
    this.setData({ mode: e.currentTarget.dataset.mode });
    this.calc();
  },
  onPickTime(e) {
    this.setData({ pickIdx: Number(e.detail.value) });
  },
  onAddr() {
    wx.navigateTo({ url: '/pages/address/address?select=1' });
  },
  onMinus(e) {
    store.changeQty(e.currentTarget.dataset.key, -1);
    this.calc();
  },
  onPlus(e) {
    store.changeQty(e.currentTarget.dataset.key, 1);
    this.calc();
  },
  onClear() {
    wx.showModal({
      title: '清空购物车',
      content: '确定清空已选的饮品吗？',
      success: (res) => {
        if (res.confirm) {
          store.saveCart([]);
          this.calc();
        }
      }
    });
  },
  onCoupon() {
    const s = store.summary();
    const base = store.round2(s.amount - this.data.activityCut - this.data.memberCut);
    const coupons = store.validCoupons();
    const items = coupons.map(c => {
      const cut = store.couponCut(c, base, s.list, this.data.mode);
      return cut ? `${c.title}  省 ¥${cut}` : `${c.title}（${c.cond}）`;
    });
    items.push('不使用优惠券');
    wx.showActionSheet({
      itemList: items,
      success: (res) => {
        if (res.tapIndex === coupons.length) {
          this.userCoupon = '';
        } else {
          const c = coupons[res.tapIndex];
          if (!store.couponCut(c, base, s.list, this.data.mode)) {
            wx.showToast({ title: '当前订单不满足使用条件', icon: 'none' });
            return;
          }
          this.userCoupon = c.id;
        }
        this.calc();
      }
    });
  },
  onPoints(e) {
    this.setData({ usePoints: e.detail.value });
    this.calc();
  },
  onRemark(e) {
    this.setData({ remark: e.detail.value });
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
      title: '钱包余额不足',
      content: `咖啡钱包余额 ¥${this.data.balance}，本单需支付 ¥${this.data.pay}`,
      confirmText: '去充值',
      success: r => { if (r.confirm) wx.navigateTo({ url: '/pages/wallet/wallet' }); }
    });
  },
  onGoMenu() {
    wx.switchTab({ url: '/pages/menu/menu' });
  },
  onPay() {
    if (this.paying || !this.data.list.length) return;
    if (this.data.mode === 'delivery' && !this.data.addr) {
      wx.showToast({ title: '请先添加收货地址', icon: 'none' });
      return;
    }
    const pay = this.data.pay;
    if (this.data.payBy === 'wallet') {
      if (this.data.balance < pay) {
        this.askRecharge();
        return;
      }
      wx.showModal({
        title: '咖啡钱包支付',
        content: `余额 ¥${this.data.balance}，本单支付 ¥${pay}`,
        confirmText: '确认支付',
        success: r => { if (r.confirm) this.place('wallet'); }
      });
      return;
    }
    this.paying = true;
    const count = this.data.list.reduce((n, x) => n + x.qty, 0);
    host.pay(pay, `拾光咖啡 · ${count} 杯饮品`).then(ok => {
      this.paying = false;
      if (ok) this.place('wechat');
      else wx.showToast({ title: '已取消支付', icon: 'none' });
    });
  },
  place(payBy) {
    const s = store.summary();
    if (!s.list.length) return;
    const d = this.data;
    const now = Date.now();
    const seq = store.nextSeq();
    const code = (d.mode === 'pickup' ? 'A' : 'W') + seq;
    const pointsUsed = d.usePoints ? d.usePointsNum : 0;
    const order = {
      id: 'SG' + now.toString().slice(-10),
      code,
      mode: d.mode,
      storeId: d.store.id,
      createdAt: now,
      // 预约单：预约时间前 5 分钟开始做
      makeAt: d.mode === 'pickup' && d.pickIdx > 0 ? this.slots[d.pickIdx] - 5 * store.MIN : 0,
      items: s.list.map(x => ({
        drinkId: x.drinkId, name: x.name, img: x.img, size: x.size, temp: x.temp, sugar: x.sugar,
        addons: x.addons || [], spec: x.spec, qty: x.qty, unit: x.unit, total: x.total
      })),
      count: s.count,
      goods: s.amount,
      activityCut: d.activityCut,
      memberCut: d.memberCut,
      couponId: d.coupon ? d.coupon.id : '',
      couponTitle: d.coupon ? d.coupon.title : '',
      couponCut: d.couponCut,
      pointsUsed,
      pointsCut: pointsUsed / 100,
      deliveryFee: d.deliveryFee,
      pay: d.pay,
      payBy,
      earn: d.earn,
      remark: d.remark,
      pickTime: d.mode === 'pickup' ? d.pickTimes[d.pickIdx] : '约 30 分钟送达',
      addr: d.mode === 'delivery' ? { name: d.addr.name, phone: d.addr.phoneMask, text: d.addr.text } : null
    };
    store.addOrder(order);
    store.saveCart([]);
    if (order.couponId) store.useCoupon(order.couponId);
    if (pointsUsed) store.changePoints(-pointsUsed, `下单抵扣 ${code}`);
    if (order.earn) store.changePoints(order.earn, `消费得积分 ${code}`);
    if (payBy === 'wallet') store.changeWallet(-order.pay, `订单消费 ${code}`, 'pay');
    this.userCoupon = undefined;
    this.setData({ remark: '', usePoints: false, pickIdx: 0 });
    wx.showToast({ title: '支付成功', icon: 'success' });
    wx.switchTab({ url: '/pages/pickup/pickup' });
  }
});
