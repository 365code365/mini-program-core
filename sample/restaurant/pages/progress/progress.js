const S = require('../../utils/store.js');
const { RATE_TAGS } = require('../../utils/data.js');
const STAR_TEXT = ['', '很差', '一般', '还行', '满意', '超赞'];

Page({
  data: {
    o: null, missing: false, steps: [], stage: 0, fill: 0, headline: '', eta: '', items: [],
    canPickup: false, active: false, canCancel: false, canUrge: false, urged: '',
    starList: [1, 2, 3, 4, 5], stars: 5, starText: STAR_TEXT[5], rateTags: [], rateText: '', rating: null
  },
  onLoad(q) {
    // 没带 id 时展示最近一笔订单
    this.id = q.id || (S.getOrders()[0] || {}).id;
    this.wantRate = q.rate === '1';
    this.setData({ rateTags: RATE_TAGS.map(name => ({ name, on: false })) });
    this.render();
    // 出餐进度按付款时间推进，每秒刷新一次（状态没变时只更新倒计时那一行）
    this.timer = setInterval(() => this.render(), 1000);
  },
  onUnload() { clearInterval(this.timer); },
  render(force) {
    const o = this.id ? S.getOrder(this.id) : null;
    if (!o) {
      if (!this.data.missing) this.setData({ missing: true, o: null });
      return;
    }
    const active = o.status === 'paid';
    const stage = o.status === 'done' ? 3 : (o.stage || 0);
    let eta = '';
    if (active && stage < 2) eta = '预计 ' + Math.max(1, Math.ceil((S.STAGE_MS[2] - (Date.now() - o.paidAt)) / 1000)) + ' 秒后可取餐';
    const patch = {};
    if (eta !== this.data.eta) patch.eta = eta;
    const key = [o.status, stage, o.urgedAt || 0, o.rating ? 1 : 0].join('|');
    if (force || key !== this.lastKey) {
      this.lastKey = key;
      const headline = o.status === 'canceled' ? '订单已取消' : o.status === 'unpaid' ? '订单待支付' :
        stage >= 3 ? '已取餐，祝您用餐愉快' : stage === 2 ? '餐品已备好，请凭取餐码取餐' : stage === 1 ? '后厨正在制作' : '订单已提交，后厨马上开做';
      Object.assign(patch, {
        o: {
          no: o.no, table: o.table, dine: o.dine, remark: o.remark || '无', tableware: o.tableware === 0 ? '不需要' : o.tableware + ' 份',
          time: S.fmtTime(o.createdAt), payText: S.yuan(o.pay), discountText: S.yuan(o.discount), packText: o.pack ? S.yuan(o.pack) : '',
          payMethod: o.payMethod || '—', status: o.status, cancelReason: o.cancelReason || '', refunded: !!o.refund
        },
        items: o.items.map(i => Object.assign({}, i, { lineText: S.yuan(i.unit * i.qty) })),
        steps: S.STAGES.concat(['已取餐']).map((t, i) => ({ t, done: i <= stage, cur: i === stage })),
        stage, fill: Math.min(100, Math.round(stage / 3 * 100)), headline,
        canPickup: active && stage === 2, active, canCancel: S.canCancel(o), canUrge: active && stage < 2,
        urged: o.urgedAt ? S.hm(o.urgedAt) : '', rating: o.rating || null
      });
      if (stage === 2 && active && !this.notified) { this.notified = true; wx.vibrateShort && wx.vibrateShort({}); }
    }
    if (Object.keys(patch).length) this.setData(patch);
    if (this.wantRate && o.status === 'done' && !o.rating) {
      this.wantRate = false;
      wx.pageScrollTo({ selector: '.rate', duration: 200 });
    }
  },
  pickup() {
    wx.showModal({
      title: '确认取餐', content: '请确认已从取餐口拿到全部餐品', confirmText: '已取餐',
      success: r => {
        if (!r.confirm) return;
        if (S.pickupOrder(this.id)) {
          wx.showToast({ title: '用餐愉快，吃完来评价一下吧', icon: 'none' });
          this.wantRate = true;
        }
        this.render(true);
      }
    });
  },
  // 催单：3 分钟内只能催一次，催过的时间显示在顶部
  urge() {
    const r = S.urgeOrder(this.id);
    wx.showToast({ title: r.ok ? '已提醒后厨加急' : r.err, icon: 'none' });
    this.render(true);
  },
  // 待支付随时取消；已付款要在后厨开做前，钱原路退回
  cancel() {
    const o = S.getOrder(this.id);
    if (!o) return;
    const paid = o.status === 'paid';
    wx.showModal({
      title: '取消订单',
      content: paid ? '后厨还没开做，取消后 ¥' + S.yuan(o.pay) + (o.payMethod === '饭卡' ? ' 立即退回饭卡' : ' 原路退回微信') : '取消后菜品不会保留，确定取消吗？',
      confirmText: '取消订单', cancelText: '再想想',
      success: r => {
        if (!r.confirm) return;
        const done = S.cancelOrder(this.id);
        wx.showToast({ title: done ? (paid ? '订单已取消，钱已退回' : '订单已取消') : '后厨已经开做，不能取消了', icon: 'none' });
        this.render(true);
      }
    });
  },
  // 加菜：回点餐页、桌号不变，加的菜单独成一单
  addMore() {
    const o = S.getOrder(this.id);
    if (o && o.table) S.setTable(o.table);
    wx.switchTab({ url: '/pages/menu/menu' });
  },
  goPay() { wx.redirectTo({ url: '/pages/pay/pay?id=' + this.id }); },
  again() {
    S.reorder(S.getOrder(this.id));
    wx.showToast({ title: '已放回购物车', icon: 'none' });
    wx.switchTab({ url: '/pages/menu/menu' });
  },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  // —— 评价 ——
  onStar(e) {
    const stars = Number(e.currentTarget.dataset.n);
    this.setData({ stars, starText: STAR_TEXT[stars] });
  },
  onRateTag(e) {
    const i = Number(e.currentTarget.dataset.i);
    this.setData({ ['rateTags[' + i + '].on']: !this.data.rateTags[i].on });
  },
  onRateText(e) { this.setData({ rateText: e.detail.value }); },
  onSubmitRate() {
    const rating = {
      stars: this.data.stars,
      tags: this.data.rateTags.filter(t => t.on).map(t => t.name),
      text: this.data.rateText.trim(),
      time: S.fmtTime(Date.now())
    };
    if (!S.rateOrder(this.id, rating)) {
      wx.showToast({ title: '这一单已经评价过了', icon: 'none' });
      return;
    }
    wx.showToast({ title: '感谢评价，后厨会看到的', icon: 'none' });
    this.render(true);
  }
});
