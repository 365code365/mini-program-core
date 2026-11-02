const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const orders = require('../../utils/orders.js');
const host = require('../../utils/host.js');

const STAR_TEXT = ['', '很差', '一般', '还行', '满意', '超赞'];

Page({
  data: {
    order: null,
    shop: {},
    st: {},
    steps: [],
    lines: [],
    createdText: '',
    payName: '',
    rider: data.RIDER,
    urged: '',
    starList: [1, 2, 3, 4, 5],
    stars: 5,
    starText: STAR_TEXT[5],
    rateTags: [],
    rateText: '',
    rating: null
  },

  onLoad(options) {
    this.orderId = options.id || '';
    this.wantRate = options.rate === '1';
    this.setData({ rateTags: data.rateTags.map(name => ({ name, on: false })) });
  },

  onShow() {
    this.load();
    clearInterval(this.timer);
    // 待支付要走倒计时（每秒）；配送中的进度按分钟推进
    this.timer = setInterval(() => this.load(), 1000);
  },

  onHide() {
    clearInterval(this.timer);
  },

  onUnload() {
    clearInterval(this.timer);
  },

  load() {
    const l = orders.getOrders();
    // 没带 id：看最近一单进行中的；带了但找不到：显示「订单不存在」
    const o = this.orderId ? l.find(x => x.id === this.orderId) : (l.find(x => x.status === 'paid') || l[0]);
    if (!o) {
      if (this.data.order !== null) this.setData({ order: null });
      return;
    }
    this.orderId = o.id;
    const st = orders.stageOf(o);
    // 状态不变的订单不必每秒重算一遍（配送中按分钟推进，看分钟数就够）
    const key = [o.status, st.stage, st.progress, o.rating ? 1 : 0, o.urgedAt || 0, st.canPay ? st.sub : ''].join('|');
    if (key === this.lastKey) return;
    this.lastKey = key;
    const shop = data.findShop(o.shopId);
    const lines = [
      { name: '商品金额', value: '¥' + o.goods },
      { name: '打包费', value: '¥' + o.packFee },
      { name: '配送费', value: '¥' + o.deliveryFee }
    ];
    if (o.fullCut) lines.push({ name: '店铺满减', value: '-¥' + o.fullCut, cut: true });
    if (o.newCut) lines.push({ name: '新客立减', value: '-¥' + o.newCut, cut: true });
    if (o.shopCut) lines.push({ name: o.shopCouponTitle || '商家代金券', value: '-¥' + o.shopCut, cut: true });
    if (o.redCut) lines.push({ name: o.redTitle || '红包', value: '-¥' + o.redCut, cut: true });
    const title = o.status === 'paid' ? '配送中' : o.status === 'unpaid' ? '待支付' : '订单详情';
    if (title !== this.navTitle) {
      this.navTitle = title;
      wx.setNavigationBarTitle({ title });
    }
    this.setData({
      order: o,
      shop,
      st,
      steps: orders.stepsOf(o),
      lines,
      createdText: store.dateTime(o.createdAt),
      payName: o.paidAt ? orders.PAY_NAME[o.payBy] : '未支付',
      urged: o.urgedAt ? store.hm(o.urgedAt) : '',
      rating: o.rating || null
    });
    if (this.wantRate && st.canRate) {
      this.wantRate = false;
      wx.pageScrollTo({ selector: '.rate', duration: 200 });
    }
  },

  reload() {
    this.lastKey = '';
    this.load();
  },

  // ───────── 支付 / 取消 / 退款 ─────────
  onPay() {
    const o = this.data.order;
    const balance = store.getWallet().balance;
    wx.showActionSheet({
      itemList: [`微信支付 ¥${o.pay}`, `余额支付（可用 ¥${balance}）`],
      success: res => {
        if (res.tapIndex === 1) {
          if (balance < o.pay) {
            wx.showModal({
              title: '余额不足',
              content: `可用余额 ¥${balance}，本单需支付 ¥${o.pay}`,
              confirmText: '去充值',
              success: r => { if (r.confirm) wx.navigateTo({ url: '/pages/wallet/wallet' }); }
            });
            return;
          }
          this.finishPay('wallet');
          return;
        }
        host.pay(o.pay, `${this.data.shop.name} · ${o.count} 件商品`).then(ok => {
          if (ok) this.finishPay('wechat');
          else wx.showToast({ title: '已取消支付', icon: 'none' });
        });
      }
    });
  },

  finishPay(payBy) {
    const r = orders.payOrder(this.orderId, payBy);
    wx.showToast({ title: r.err || '支付成功', icon: r.err ? 'none' : 'success' });
    this.reload();
  },

  onCancel() {
    const o = this.data.order;
    wx.showModal({
      title: '取消订单',
      content: o.paidAt ? `商家还没接单，取消后 ¥${o.pay} 立即原路退回，红包一并退回` : '取消后订单使用的红包会退回账户',
      confirmText: '确认取消',
      cancelText: '再想想',
      success: r => {
        if (!r.confirm) return;
        wx.showToast({ title: orders.cancelOrder(o.id) ? '订单已取消' : '商家已接单，请申请退款', icon: 'none' });
        this.reload();
      }
    });
  },

  // 申请退款：先选原因，再确认退多少、退到哪（演示里商家秒同意）
  onRefund() {
    const reasons = data.refundReasons;
    wx.showActionSheet({
      itemList: reasons,
      success: res => {
        const reason = reasons[res.tapIndex];
        const o = this.data.order;
        wx.showModal({
          title: '申请退款',
          content: `原因：${reason}\n¥${o.pay} 将原路退回${o.payBy === 'wallet' ? '余额' : '微信'}，红包一并退回，本单吃货豆扣回`,
          confirmText: '确认退款',
          cancelText: '再想想',
          success: r => {
            if (!r.confirm) return;
            const done = orders.refundOrder(this.orderId, reason);
            wx.showToast({ title: done ? `退款成功，¥${done.pay} 已原路退回` : '订单已送达，无法退款', icon: 'none' });
            this.reload();
          }
        });
      }
    });
  },

  onUrge() {
    if (orders.urge(this.orderId)) wx.showToast({ title: '已催单，商家和骑手会尽快处理', icon: 'none' });
    else wx.showToast({ title: '刚刚催过了，5 分钟后可再催', icon: 'none' });
    this.reload();
  },

  onConfirm() {
    wx.showModal({
      title: '确认收货',
      content: '确认已经收到全部餐品了吗？',
      success: (res) => {
        if (!res.confirm) return;
        orders.confirmOrder(this.orderId);
        wx.showToast({ title: '已确认收货，来评价一下吧', icon: 'none' });
        this.wantRate = true;
        this.reload();
      }
    });
  },

  // ───────── 联系 ─────────
  onCallRider() {
    host.call(this.data.rider.phone, `联系骑手${this.data.rider.name}`);
  },

  onCallShop() {
    host.call(this.data.shop.phone, '联系商家');
  },

  // 联系客服：按订单状态给能做的事
  onService() {
    const st = this.data.st;
    const acts = [];
    if (st.canUrge) acts.push({ t: '催单', fn: () => this.onUrge() });
    if (st.canCancel && this.data.order.paidAt) acts.push({ t: '取消订单', fn: () => this.onCancel() });
    if (st.canRefund) acts.push({ t: '申请退款', fn: () => this.onRefund() });
    acts.push({ t: '联系商家', fn: () => this.onCallShop() });
    if (st.active && st.stage >= 1) acts.push({ t: '联系骑手', fn: () => this.onCallRider() });
    acts.push({ t: '常见问题', fn: () => wx.navigateTo({ url: '/pages/service/service' }) });
    acts.push({ t: `客服热线 ${data.SERVICE_PHONE}`, fn: () => host.call(data.SERVICE_PHONE, '饱了么客服') });
    const list = acts.slice(0, 6);
    wx.showActionSheet({
      itemList: list.map(a => a.t),
      success: res => list[res.tapIndex].fn()
    });
  },

  onCopy() {
    host.copy(this.data.order.id);
  },

  onShop() {
    wx.navigateTo({ url: `/pages/shop/shop?id=${this.data.order.shopId}` });
  },

  onAgain() {
    const n = orders.reorder(this.data.order);
    wx.showToast({ title: n ? `已把 ${n} 件商品放回购物车` : '商品已下架', icon: 'none' });
    this.onShop();
  },

  // ───────── 评价 ─────────
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
    if (!orders.rateOrder(this.orderId, rating)) {
      wx.showToast({ title: '这单已经评价过了', icon: 'none' });
      return;
    }
    wx.showToast({ title: '感谢评价，吃货豆 +20', icon: 'none' });
    this.reload();
  }
});
