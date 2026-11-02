const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const STAR_TEXT = ['', '很差', '一般', '还行', '满意', '超赞'];
Page({
  data: {
    order: null,
    st: null,
    store: data.stores[0],
    steps: [],
    time: '',
    lines: [],
    payName: '',
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
    this.setData({ rateTags: data.rateTags.map(name => ({ name, on: false })) });
  },
  onShow() {
    this.load();
    clearInterval(this.timer);
    this.timer = setInterval(() => this.load(), 15000);
  },
  onHide() {
    clearInterval(this.timer);
  },
  onUnload() {
    clearInterval(this.timer);
  },
  load() {
    const o = store.findOrder(this.id);
    if (!o) {
      this.setData({ order: null });
      return;
    }
    const st = store.stageOf(o);
    // 价格明细：只列这一单真有的项（老订单没有这些字段，按 0 处理）
    const lines = [{ name: '饮品金额', value: '¥' + (o.goods || store.round2(o.items.reduce((n, i) => n + i.total, 0))) }];
    if (o.activityCut) lines.push({ name: '新品第 2 杯半价', value: '-¥' + o.activityCut, cut: true });
    if (o.memberCut) lines.push({ name: '会员日 8 折', value: '-¥' + o.memberCut, cut: true });
    if (o.couponCut) lines.push({ name: o.couponTitle || '优惠券', value: '-¥' + o.couponCut, cut: true });
    if (o.pointsCut) lines.push({ name: `积分抵扣（${o.pointsUsed} 积分）`, value: '-¥' + o.pointsCut, cut: true });
    if (o.mode === 'delivery') lines.push({ name: '配送费', value: o.deliveryFee ? '¥' + o.deliveryFee : '¥0' });
    this.setData({
      order: o,
      st,
      store: data.findStore(o.storeId),
      steps: st.stage >= 0 ? st.names.map((name, i) => ({ name, done: i <= st.stage })) : [],
      time: store.dateTime(o.createdAt),
      lines,
      payName: o.payBy === 'wallet' ? '咖啡钱包' : '微信支付',
      rating: o.rating || null
    });
    if (this.wantRate && st.stage === 3 && !o.rating) {
      this.wantRate = false;
      wx.pageScrollTo({ selector: '.rate', duration: 200 });
    }
  },
  onCopy() {
    host.copy(this.data.order.id);
  },
  onCall() {
    host.call(this.data.store.phone);
  },
  onNav() {
    host.openMap(this.data.store);
  },
  onCancel() {
    const o = this.data.order;
    wx.showModal({
      title: '取消订单',
      content: `取消后 ¥${o.pay} 将原路退回${o.payBy === 'wallet' ? '咖啡钱包' : '微信'}，优惠券与积分一并退回`,
      confirmText: '确认取消',
      cancelText: '再想想',
      success: r => {
        if (!r.confirm) return;
        wx.showToast({ title: store.cancelOrder(o.id) ? '订单已取消' : '咖啡师已开始制作，无法取消', icon: 'none' });
        this.load();
      }
    });
  },
  onConfirm() {
    store.updateOrder(this.id, { status: 'done', doneAt: Date.now() });
    wx.showToast({ title: '订单已完成，来评价一下吧', icon: 'none' });
    this.load();
  },
  onAgain() {
    if (store.reorder(this.data.order)) wx.navigateTo({ url: '/pages/cart/cart' });
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
    store.updateOrder(this.id, { rating });
    store.changePoints(20, `评价奖励 ${this.data.order.code}`);
    wx.showToast({ title: '感谢评价，+20 积分', icon: 'none' });
    this.load();
  }
});
