const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const slots = require('../../utils/slots.js');

const FREE_SHIP = 49, SHIP_FEE = 6;

function mask(p) { return p.slice(0, 3) + '****' + p.slice(7); }

Page({
  data: {
    addr: null, sum: { lines: [], total: 0, count: 0 }, addons: data.ADDONS.map(a => Object.assign({ on: a.key === 'bag' }, a)),
    addonFee: 0, shipFee: 0, coupon: null, usableN: 0, payable: 0, remark: '',
    showTime: false, days: [], slots: [], pv: [0, 0], timeText: '', todayTip: ''
  },

  onLoad() {
    this.picked = null;
    this.couponTouched = false;
  },

  onShow() {
    const a = store.selectedAddr();
    const sum = store.cartSummary();
    if (!sum.count) {
      wx.showToast({ title: '购物篮是空的', icon: 'none' });
      setTimeout(() => wx.navigateBack(), 600);
      return;
    }
    this.setData({ addr: a ? Object.assign({}, a, { phoneMask: mask(a.phone) }) : null, sum });
    this.recalc();
  },

  // 价格：衣物 + 加购 + 上门费（衣物满 49 免）- 券；没手动选过券时自动挑面额最大的可用券
  recalc(couponId) {
    const sum = this.data.sum;
    const addonFee = this.data.addons.filter(a => a.on).reduce((s, a) => s + a.price, 0);
    const shipFee = sum.total >= FREE_SHIP ? 0 : SHIP_FEE;
    const usable = store.coupons().filter(c => c.state === 'ok' && sum.total >= c.min && c.scope === '全场通用');
    let coupon = this.data.coupon;
    if (couponId !== undefined) coupon = usable.find(c => c.id === couponId) || null;
    else if (!this.couponTouched) coupon = usable.slice().sort((a, b) => b.amount - a.amount)[0] || null;
    else if (coupon && !usable.find(c => c.id === coupon.id)) coupon = null;
    const payable = Math.max(0.01, sum.total + addonFee + shipFee - (coupon ? coupon.amount : 0));
    this.usable = usable;
    this.setData({ addonFee, shipFee, coupon, usableN: usable.length, payable: Math.round(payable * 100) / 100 });
  },

  pickAddr() { wx.navigateTo({ url: '/pages/address/address?select=1' }); },
  editItems() { wx.navigateBack(); },

  // ───── 取件时段：picker-view 两列联动 ─────
  openTime() {
    const days = slots.days();
    let pv = [0, 0];
    if (this.picked) {
      const d = days.findIndex(x => x.date === this.picked.date);
      if (d >= 0) {
        const s = slots.slotsFor(days[d]).indexOf(this.picked.slot);
        pv = [d, Math.max(0, s)];
      }
    }
    const today = days[0].offset === 0;
    this.setData({
      showTime: true, days, pv, slots: slots.slotsFor(days[pv[0]]),
      todayTip: today ? '今天已过的时段不可预约（需提前 1 小时）' : '今天的时段已约满，最早明天上门'
    });
  },

  onPv(e) {
    const v = e.detail.value;
    let d = v[0], s = v[1];
    let list = this.data.slots;
    if (d !== this.data.pv[0]) {
      // 换了日期：第二列换成那天的时段，下标夹到范围内
      list = slots.slotsFor(this.data.days[d]);
      s = Math.min(s, list.length - 1);
    }
    this.setData({ pv: [d, s], slots: list });
  },

  closeTime() { this.setData({ showTime: false }); },

  confirmTime() {
    const day = this.data.days[this.data.pv[0]];
    const slot = this.data.slots[this.data.pv[1]];
    this.picked = { date: day.date, label: day.label, slot };
    this.setData({ showTime: false, timeText: day.label.split(' ')[0] + ' ' + day.label.split(' ')[1] + ' ' + slot });
  },

  onAddons(e) {
    const on = e.detail.value;
    this.setData({ addons: this.data.addons.map(a => Object.assign({}, a, { on: on.indexOf(a.key) >= 0 })) });
    this.recalc();
  },

  pickCoupon() {
    const list = this.usable || [];
    if (!list.length) return wx.showToast({ title: '暂无可用优惠券', icon: 'none' });
    const top = list.slice(0, 5);
    wx.showActionSheet({
      itemList: top.map(c => c.title + '（-¥' + c.amount + '）').concat(['不使用优惠券']),
      success: res => {
        this.couponTouched = true;
        this.recalc(res.tapIndex < top.length ? top[res.tapIndex].id : null);
      }
    });
  },

  onRemark(e) { this.setData({ remark: e.detail.value }); },

  submit() {
    if (!this.data.addr) return wx.showToast({ title: '请先添加取件地址', icon: 'none' });
    if (!this.picked) return this.openTime();
    // 选的时段可能在停留期间过期了（例如今天 11 点前选了 11-13 点，现在已经 10:30 以后）
    const days = slots.days();
    const day = days.find(d => d.date === this.picked.date);
    if (!day || slots.slotsFor(day).indexOf(this.picked.slot) < 0) {
      this.picked = null;
      this.setData({ timeText: '' });
      return wx.showModal({ title: '时段已过期', content: '所选取件时段已不可预约，请重新选择', showCancel: false, success: () => this.openTime() });
    }
    wx.showLoading({ title: '正在下单', mask: true });
    const ts = String(Math.floor(Date.now() / 1000));
    wx.requestPayment({
      timeStamp: ts, nonceStr: 'jy' + ts, package: 'prepay_id=wx' + ts, signType: 'RSA', paySign: 'demo',
      success: () => { wx.hideLoading(); this.createOrder('wxpay'); },
      fail: err => {
        wx.hideLoading();
        // 用户在收银台点了取消：留在本页
        if (/fail cancel$/.test(err.errMsg || '')) return wx.showToast({ title: '已取消支付', icon: 'none' });
        // 拿不到微信支付（桌面端 / 未接宿主）：给一个需要用户再确认的模拟收银台
        wx.showModal({
          title: '模拟支付',
          content: '当前环境无法唤起微信支付。演示环境下可模拟支付 ¥' + this.data.payable + '，不会真实扣款。',
          confirmText: '确认支付',
          success: res => {
            if (res.confirm) this.createOrder('mock');
            else wx.showToast({ title: '已取消支付', icon: 'none' });
          }
        });
      }
    });
  },

  createOrder(payBy) {
    const d = this.data;
    const o = {
      id: store.newOrderId(), bagNo: store.newBagNo(), createdAt: Date.now(), payBy, step: 0, stepAt: [Date.now()],
      lines: d.sum.lines, count: d.sum.count, goods: d.sum.total, addons: d.addons.filter(a => a.on).map(a => ({ name: a.name, price: a.price })),
      addonFee: d.addonFee, shipFee: d.shipFee, coupon: d.coupon ? { id: d.coupon.id, title: d.coupon.title, amount: d.coupon.amount } : null,
      payable: d.payable, remark: d.remark.trim(), pickup: this.picked, addr: d.addr, cancelled: false, review: null
    };
    store.saveOrder(o);
    if (d.coupon) store.useCoupon(d.coupon.id, true);
    store.clearCart();
    wx.redirectTo({ url: '/pages/pay-result/pay-result?id=' + o.id });
  }
});
