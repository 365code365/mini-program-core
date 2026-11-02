const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const STAR_TEXT = ['', '很差', '一般', '还行', '满意', '超赞'];
const STEPS = ['已下单', '花艺制作', '配送中', '已签收'];
Page({
  data: {
    o: null, st: null, steps: [], items: [], lines: [], courier: data.COURIER,
    starList: [1, 2, 3, 4, 5], stars: 5, starText: STAR_TEXT[5], tags: [], rateText: ''
  },
  onLoad(q) {
    this.no = String(q.no || '');
    this.wantRate = String(q.rate) === '1';
    this.setData({ tags: data.rateTags.map(n => ({ n, on: false })) });
  },
  onShow() {
    this.load();
    clearInterval(this.timer);
    this.timer = setInterval(() => this.load(), 30000);
  },
  onHide() { clearInterval(this.timer); },
  onUnload() { clearInterval(this.timer); },
  load() {
    const o = store.findOrder(this.no);
    if (!o) { this.setData({ o: null }); return; }
    const st = store.stageOf(o);
    const items = o.items.map(x => {
      const it = store.expand(x);
      // 历史订单按下单时的单价算
      if (x.unit) { it.unit = x.unit; it.sum = x.unit * x.qty; }
      return it;
    });
    const lines = [{ n: '商品金额', v: '¥' + (o.goods || items.reduce((s, x) => s + x.sum, 0)) }];
    if (o.activityCut) lines.push({ n: '向日葵第二束半价', v: '-¥' + o.activityCut, cut: true });
    if (o.memberCut) lines.push({ n: o.memberName || '会员折扣', v: '-¥' + o.memberCut, cut: true });
    if (o.couponCut) lines.push({ n: o.couponTitle || '优惠券', v: '-¥' + o.couponCut, cut: true });
    lines.push({ n: '配送费', v: '免运费' });
    this.setData({
      o, st, items, lines,
      steps: st.step ? STEPS.map((n, i) => ({ n, on: i <= st.step })) : []
    });
    if (this.wantRate && st.canRate) {
      this.wantRate = false;
      wx.pageScrollTo({ selector: '.rate', duration: 200 });
    }
  },
  onCancel() {
    const o = this.data.o;
    wx.showModal({
      title: '取消订单',
      content: '取消后 ¥' + o.total + ' 将原路退回微信' + (o.couponTitle ? '，「' + o.couponTitle + '」券一并退回' : ''),
      confirmText: '确认取消', cancelText: '再想想', confirmColor: '#ef4f76',
      success: r => {
        if (!r.confirm) return;
        wx.showToast({ title: store.cancelOrder(o.no) ? '已取消，退款 1-3 天到账' : '花艺师已开始制作，无法取消', icon: 'none' });
        this.load();
      }
    });
  },
  onCourier() { host.call(data.COURIER.phone, '配送员' + data.COURIER.name); },
  onService() { host.call(data.SERVICE_PHONE, '花语鲜花客服'); },
  onAgain() {
    store.reorder(this.data.o);
    wx.switchTab({ url: '/pages/cart/cart' });
  },
  onCopy() { host.copy(this.data.o.no); },
  onProduct(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  onStar(e) {
    const stars = Number(e.currentTarget.dataset.n);
    this.setData({ stars, starText: STAR_TEXT[stars] });
  },
  onTag(e) {
    const i = Number(e.currentTarget.dataset.i);
    this.setData({ ['tags[' + i + '].on']: !this.data.tags[i].on });
  },
  onRateText(e) { this.setData({ rateText: e.detail.value }); },
  onSubmitRate() {
    const rating = {
      stars: this.data.stars,
      tags: this.data.tags.filter(t => t.on).map(t => t.n),
      text: this.data.rateText.trim(),
      time: store.dateTime(Date.now())
    };
    store.updateOrder(this.no, { rating });
    wx.showToast({ title: '感谢评价，花艺师会看到的', icon: 'none' });
    this.load();
  }
});
