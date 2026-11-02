// 支付：行程结束后的车费（可换券）或取消费。钱包余额够就默认钱包，不够只能微信支付或先去充值；
// 微信支付走 host.pay（宿主没有支付能力时页内确认）。付完跳行程详情（车费单顺带提示评价）
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');

function fareLines(t) {
  const f = t.fare;
  const lines = [
    { name: '起步价', value: '¥' + f.base },
    { name: '里程费（' + t.km + ' 公里）', value: '¥' + f.mileage },
    { name: '时长费（' + t.min + ' 分钟）', value: '¥' + f.time }
  ];
  if (f.far) lines.push({ name: '远途费（超出 ' + data.FAR_KM + ' 公里部分）', value: '¥' + f.far });
  if (f.discount) lines.push({ name: '拼成 82 折', value: '-¥' + f.discount, cut: true });
  return lines;
}

Page({
  data: {
    trip: null, paid: false, lines: [], coupons: [], couponId: '', couponName: '', cut: 0,
    due: 0, balance: 0, enough: true, payBy: 'wallet', sheet: false, cancelWhy: ''
  },
  onLoad(q) {
    this.id = q.id || '';
    this.first = true;
  },
  onShow() {
    this.load();
  },
  load() {
    const t = store.findTrip(this.id);
    if (!t) {
      this.setData({ trip: null });
      return;
    }
    if (t.status !== 'unpaid') {
      this.setData({ trip: t, paid: true });
      return;
    }
    const balance = store.getWallet().balance;
    let couponId = this.first ? t.couponId || '' : this.data.couponId;
    let cut = 0;
    let coupons = [];
    let lines;
    if (t.cancelled) {
      lines = [{ name: '取消费', value: '¥' + t.cancelFee }];
    } else {
      const usable = store.usableCoupons({ typeKey: t.typeKey, from: t.from, to: t.to, amount: t.fare.total, at: t.when || t.createdAt });
      const hit = usable.find(x => x.coupon.id === couponId);
      if (!hit) couponId = '';
      cut = hit ? hit.cut : 0;
      coupons = usable.map(x => ({ id: x.coupon.id, name: x.coupon.name, cond: x.coupon.cond, scope: x.coupon.scope, cut: x.cut }));
      lines = fareLines(t);
    }
    const due = t.cancelled ? t.cancelFee : store.round1(t.fare.total - cut);
    const enough = balance >= due;
    // 第一次进来按余额挑；从充值页回来保留用户选的，余额还不够才换成微信
    let payBy = this.first ? (enough ? 'wallet' : 'wechat') : this.data.payBy;
    if (payBy === 'wallet' && !enough) payBy = 'wechat';
    this.first = false;
    const hitName = coupons.find(c => c.id === couponId);
    this.setData({
      trip: t, paid: false, lines, coupons, couponId, cut, couponName: hitName ? hitName.name : '',
      due, balance, enough, payBy,
      cancelWhy: t.cancelled ? (t.cancelStage === 'arrived' ? '司机到达后取消' : '接单超过 2 分钟后取消') : ''
    });
  },
  onPayBy(e) {
    const k = e.currentTarget.dataset.k;
    if (k === 'wallet' && !this.data.enough) {
      wx.showToast({ title: '余额不足，请先充值或用微信支付', icon: 'none' });
      return;
    }
    this.setData({ payBy: k });
  },
  onRecharge() {
    wx.navigateTo({ url: '/pages/wallet/wallet' });
  },
  onCoupon() {
    if (this.data.trip.cancelled) return;
    if (!this.data.coupons.length) {
      wx.showToast({ title: '这一单没有满足条件的优惠券', icon: 'none' });
      return;
    }
    this.setData({ sheet: true });
  },
  onCloseSheet() {
    this.setData({ sheet: false });
  },
  noop() {},
  onPickCoupon(e) {
    this.setData({ couponId: String(e.currentTarget.dataset.id || ''), sheet: false });
    this.load();
  },
  onPay() {
    if (this.paying) return;
    const d = this.data;
    const t = d.trip;
    const desc = t.cancelled ? '滴达出行 取消费' : '滴达出行 ' + t.type + ' ' + t.from.name + ' → ' + t.to.name;
    const done = () => {
      const r = store.payTrip(this.id, d.payBy, d.couponId);
      if (!r) {
        wx.showToast({ title: '支付失败，请刷新后重试', icon: 'none' });
        this.load();
        return;
      }
      wx.showToast({ title: '支付成功', icon: 'success' });
      setTimeout(() => wx.redirectTo({ url: '/pages/trip-detail/trip-detail?id=' + this.id + (t.cancelled ? '' : '&rate=1') }), 600);
    };
    if (d.payBy === 'wallet') {
      wx.showModal({
        title: '钱包支付',
        content: '使用钱包余额支付 ¥' + d.due + '，支付后余额 ¥' + store.round2(d.balance - d.due),
        confirmText: '确认支付',
        success: r => {
          if (r.confirm) done();
        }
      });
      return;
    }
    this.paying = true;
    host.pay(d.due, desc).then(ok => {
      this.paying = false;
      if (ok) done();
      else wx.showToast({ title: '已取消支付', icon: 'none' });
    });
  },
  onDetail() {
    wx.redirectTo({ url: '/pages/trip-detail/trip-detail?id=' + this.id });
  },
  onHome() {
    wx.switchTab({ url: '/pages/index/index' });
  }
});
