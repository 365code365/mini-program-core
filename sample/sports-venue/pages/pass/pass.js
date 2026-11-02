// 入场码：canvas 2d 画票码，每 30 秒按时间片换一次（截图转发的旧码会失效），下方同时给 6 位核销码
const store = require('../../utils/store.js');
const ORD = require('../../utils/order.js');
const CODE = require('../../utils/code.js');
const SIZE = 200;
const PERIOD = 30;
Page({
  data: { o: null, sec: PERIOD, size: SIZE, valid: false },
  onLoad(q) {
    const raw = store.getOrder(q.id);
    if (!raw) return;
    const o = ORD.view(raw);
    this.setData({ o, valid: o.st === 'paid' });
    // 出示码时别让屏幕自己熄掉；没有宿主能力就算了
    wx.setKeepScreenOn({ keepScreenOn: true, fail: () => {} });
  },
  onReady() { if (this.data.o) this.initCanvas(); },
  onShow() { if (this.canvas) this.start(); },
  onHide() { this.stop(); },
  onUnload() { this.stop(); wx.setKeepScreenOn({ keepScreenOn: false, fail: () => {} }); },
  initCanvas() {
    wx.createSelectorQuery().select('#qr').fields({ node: true, size: true }).exec((res) => {
      if (!res || !res[0] || !res[0].node) return;
      this.canvas = res[0].node;
      this.draw();
      this.start();
    });
  },
  slice() { return Math.floor(Date.now() / (PERIOD * 1000)); },
  draw() {
    const o = this.data.o;
    // 时间片 + 核销码 + 订单号一起散列：同一时间片内码不变
    CODE.draw(this.canvas, SIZE, o.id + ':' + o.code + ':' + this.slice(), !this.data.valid);
  },
  start() {
    this.stop();
    if (!this.data.valid) return;
    this.timer = setInterval(() => {
      const sec = PERIOD - Math.floor(Date.now() / 1000) % PERIOD;
      if (sec === PERIOD || sec > this.data.sec) this.draw();
      this.setData({ sec });
    }, 1000);
  },
  stop() { if (this.timer) { clearInterval(this.timer); this.timer = null; } },
  refresh() { if (this.canvas && this.data.valid) { this.draw(); wx.showToast({ title: '已刷新', icon: 'none' }); } },
  checkin() {
    wx.showModal({
      title: '确认已入场', content: '前台已核验入场码？确认后此码失效，不能再次使用。', confirmText: '已入场',
      success: (r) => {
        if (!r.confirm) return;
        const raw = store.getOrder(this.data.o.id);
        raw.status = 'used'; raw.usedAt = Date.now();
        store.upsertOrder(raw);
        this.stop();
        this.setData({ o: ORD.view(raw), valid: false });
        this.draw();
      }
    });
  },
  toOrder() { wx.redirectTo({ url: '/pages/order/order?id=' + this.data.o.id }); },
  toOrders() { wx.redirectTo({ url: '/pages/orders/orders' }); }
});
