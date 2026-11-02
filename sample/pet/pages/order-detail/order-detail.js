const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const flows = require('../../utils/flows.js');
const STAR_TEXT = ['', '很差', '一般', '还行', '满意', '超赞'];
const GOODS_STEPS = ['已下单', '已付款', '已发货', '已签收'];
const SERVICE_STEPS = ['已预约', '到店服务', '已完成'];
// 订单详情：商城订单与服务预约共用一页（?id=），?rate=1 打开时滚到评价
Page({
  data: {
    o: null, st: null, steps: [], lines: [], store: null, time: '', paidTime: '',
    starList: [1, 2, 3, 4, 5], stars: 5, starText: STAR_TEXT[5], rateText: '', formSeq: 0
  },
  onLoad(q) {
    this.id = q.id;
    this.wantRate = q.rate === '1';
  },
  onShow() {
    this.load();
    clearInterval(this.timer);
    this.timer = setInterval(() => this.load(), 15000);
  },
  onHide() { clearInterval(this.timer); },
  onUnload() { clearInterval(this.timer); },
  load() {
    store.tick();
    const o = store.findOrder(this.id);
    if (!o) {
      this.setData({ o: null });
      return;
    }
    const st = store.stateOf(o);
    const goods = o.kind === 'goods';
    let stage = 0;
    if (goods) stage = { unpaid: 0, paid: 1, shipped: 2, done: 3 }[o.status];
    else stage = o.status === 'done' ? 2 : 0;
    const names = goods ? GOODS_STEPS : SERVICE_STEPS;
    const lines = goods
      ? [{ k: '商品金额', v: '¥' + o.amount }, { k: '运费', v: o.freight ? '¥' + o.freight : '免运费' }]
      : [{ k: '服务金额', v: '¥' + o.base }].concat(
        o.pickup ? [{ k: '上门接送', v: '¥' + o.pickupFee }] : [],
        o.newbieCut ? [{ k: '新客洗护首单立减', v: '-¥' + o.newbieCut, cut: true }] : [],
        o.cardCut ? [{ k: '洗护次卡抵扣 1 次', v: '-¥' + o.cardCut, cut: true }] : []);
    this.setData({
      o, st, lines,
      steps: o.status === 'cancel' ? [] : names.map((name, i) => ({ name, done: i <= stage })),
      store: goods ? null : data.findStore(o.storeId),
      time: store.dateTime(o.createdAt),
      paidTime: o.paidAt ? store.dateTime(o.paidAt) : ''
    });
    if (this.wantRate && st.canRate) {
      this.wantRate = false;
      wx.pageScrollTo({ selector: '.rate', duration: 200 });
    }
  },
  onPay() { flows.pay(this.data.o).then(() => this.load()); },
  onCancel() { flows.cancel(this.data.o).then(() => this.load()); },
  onConfirm() { flows.confirm(this.data.o).then(() => this.load()); },
  onAgain() { flows.again(this.data.o); },
  onCopy() { host.copy(this.data.o.id); },
  onCallStore() { host.call(this.data.store.phone, this.data.store.name); },
  onNav() { host.openMap(this.data.store); },
  onService() { host.call(data.SERVICE_PHONE, '萌宠家客服（9:00-22:00）'); },
  onStar(e) {
    const stars = Number(e.currentTarget.dataset.n);
    this.setData({ stars, starText: STAR_TEXT[stars] });
  },
  onRateText(e) { this.setData({ rateText: e.detail.value }); },
  submitRate() {
    store.rateOrder(this.id, { stars: this.data.stars, text: this.data.rateText.trim() });
    this.setData({ rateText: '', formSeq: this.data.formSeq + 1 });
    wx.showToast({ title: '感谢评价', icon: 'success' });
    this.load();
  }
});
