const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const orders = require('../../utils/orders.js');
const host = require('../../utils/host.js');

const STAR_TEXT = ['', '很差', '一般', '还行', '满意', '超赞'];
const HOTLINE = '400-610-8899';

Page({
  data: {
    order: null,
    st: null,
    lines: [],
    items: [],
    afterSales: [],
    time: '',
    payName: '',
    rider: orders.RIDER,
    starList: [1, 2, 3, 4, 5],
    stars: 5,
    starText: STAR_TEXT[5],
    rateTags: [],
    rateText: '',
    rating: null
  },

  onLoad(q) {
    this.id = q.id;
    this.wantRate = q.rate === '1';
    this.wantPay = q.pay === '1';
    this.setData({ rateTags: data.rateTags.map(name => ({ name, on: false })) });
  },

  onShow() {
    this.load();
    clearInterval(this.timer);
    this.timer = setInterval(() => this.tick(), 1000);
  },

  onHide() {
    clearInterval(this.timer);
  },

  onUnload() {
    clearInterval(this.timer);
  },

  load() {
    const o = orders.findOrder(this.id);
    if (!o) {
      this.setData({ order: null });
      return;
    }
    const st = orders.stageOf(o);
    const lines = [
      { name: '商品金额', value: '¥' + o.goods },
      { name: '运费', value: o.fee ? '¥' + o.fee : '免运费' }
    ];
    if (o.couponCut) lines.push({ name: o.couponTitle || '优惠券', value: '-¥' + o.couponCut, cut: true });
    if (o.beansCut) lines.push({ name: `鲜豆抵扣（${o.beansUsed} 鲜豆）`, value: '-¥' + o.beansCut, cut: true });
    this.stKey = st.key;
    this.setData({
      order: o,
      st,
      lines,
      items: orders.refundable(o),
      afterSales: o.afterSales || [],
      time: store.dateTime(o.createdAt),
      payName: o.paidAt ? (o.payBy === 'wallet' ? '余额支付' : '微信支付') : '未支付',
      rating: o.rating || null
    });
    if (this.wantRate && st.canRate) {
      this.wantRate = false;
      wx.pageScrollTo({ selector: '.rate', duration: 200 });
    }
    if (this.wantPay && st.canPay) {
      this.wantPay = false;
      this.onPay();
    }
  },

  // 每秒：待付款刷新倒计时；进度换了档（拣货 → 配送 → 送达、超时关闭）整页重读
  tick() {
    const o = this.data.order;
    if (!o) return;
    const st = orders.stageOf(o);
    if (st.key !== this.stKey || (st.key === 'unpaid' && st.left <= 0)) this.load();
    else if (st.key === 'unpaid') this.setData({ st });
  },

  onPay() {
    const o = this.data.order;
    const balance = store.getWallet().balance;
    wx.showActionSheet({
      itemList: [`微信支付 ¥${o.pay}`, `余额支付（可用 ¥${balance}）`],
      success: res => {
        if (res.tapIndex === 0) {
          host.pay(o.pay, `鲜到家超市 · ${o.count} 件商品`).then(ok => this.afterPay(ok, 'wechat'));
          return;
        }
        if (balance < o.pay) {
          wx.showModal({
            title: '余额不足',
            content: `账户余额 ¥${balance}，本单需支付 ¥${o.pay}`,
            confirmText: '去充值',
            success: r => { if (r.confirm) wx.navigateTo({ url: '/pages/wallet/wallet' }); }
          });
          return;
        }
        wx.showModal({
          title: '余额支付',
          content: `账户余额 ¥${balance}，本单支付 ¥${o.pay}`,
          confirmText: '确认支付',
          success: r => { if (r.confirm) this.afterPay(true, 'wallet'); }
        });
      }
    });
  },

  afterPay(ok, payBy) {
    if (!ok) {
      wx.showToast({ title: '已取消支付', icon: 'none' });
      return;
    }
    if (!orders.markPaid(this.id, payBy)) {
      wx.showToast({ title: '订单已关闭或余额不足', icon: 'none' });
    } else {
      wx.showToast({ title: '支付成功', icon: 'success' });
    }
    this.load();
  },

  onCancel() {
    const o = this.data.order;
    const paid = o.status === 'paid';
    wx.showModal({
      title: '取消订单',
      content: paid ? `取消后 ¥${o.pay} 将原路退回${o.payBy === 'wallet' ? '余额' : '微信'}，优惠券与鲜豆一并退回` : '取消后优惠券与鲜豆将退回账户',
      confirmText: '确认取消',
      cancelText: '再想想',
      success: r => {
        if (!r.confirm) return;
        wx.showToast({ title: orders.cancelOrder(o.id) ? '订单已取消' : '商品已出库，无法取消', icon: 'none' });
        this.load();
      }
    });
  },

  onConfirm() {
    orders.confirmOrder(this.id);
    wx.showToast({ title: '已确认收货，来评价一下吧', icon: 'none' });
    this.load();
  },

  onAfter() {
    wx.navigateTo({ url: '/pages/aftersale/aftersale?id=' + this.id });
  },

  onAgain() {
    const r = orders.reorder(this.data.order);
    if (!r.lines) {
      wx.showToast({ title: '商品已下架或超出限购', icon: 'none' });
      return;
    }
    wx.switchTab({ url: '/pages/cart/cart' });
  },

  onCallRider() {
    host.call(orders.RIDER.phone);
  },

  onService() {
    host.call(HOTLINE);
  },

  onCopy() {
    host.copy(this.data.order.id);
  },

  onGoods(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  },

  onStar(e) {
    const stars = Number(e.currentTarget.dataset.n);
    this.setData({ stars, starText: STAR_TEXT[stars] });
  },

  onRateTag(e) {
    const i = Number(e.currentTarget.dataset.i);
    this.setData({ [`rateTags[${i}].on`]: !this.data.rateTags[i].on });
  },

  onRateText(e) {
    this.setData({ rateText: e.detail.value });
  },

  onSubmitRate() {
    const rating = {
      stars: this.data.stars,
      tags: this.data.rateTags.filter(t => t.on).map(t => t.name),
      text: this.data.rateText.trim(),
      time: store.dateTime(Date.now())
    };
    if (!orders.rateOrder(this.id, rating)) return;
    wx.showToast({ title: '感谢评价，+20 鲜豆', icon: 'none' });
    this.load();
  }
});
