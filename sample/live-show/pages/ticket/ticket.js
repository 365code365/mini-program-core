// 订单详情 + 电子票：待支付（倒计时、去支付、取消）、待观演（票码、出示调亮、退票）、已退票 / 已取消
const F = require('../../utils/fmt.js');
const store = require('../../utils/store.js');
const payment = require('../../utils/pay.js');
const code = require('../../utils/ticketcode.js');

const QR = 200;

Page({
  data: { o: null, statusText: '', idx: 0, tk: null, bright: false, left: '', created: '', qr: QR },
  onLoad(q) {
    this.oid = q.id;
    this.load();
  },
  goWallet() { wx.switchTab({ url: '/pages/wallet/wallet' }); },
  onShow() { if (this.data.o && this.data.o.status === 'unpaid') this.startTimer(); },
  onHide() { this.restore(); this.stopTimer(); },
  onUnload() { this.restore(); this.stopTimer(); },
  load() {
    let o = store.getOrder(this.oid);
    if (!o) { this.setData({ o: null, missing: true }); return; }
    // 待支付超时：自动取消并释放座位
    if (o.status === 'unpaid' && Date.now() > o.expireAt) { o.status = 'cancelled'; o.cancelReason = '超时未支付'; store.upsertOrder(o); }
    this.setData({ o, statusText: F.STATUS[o.status], created: F.dateTime(o.createdAt), idx: Math.min(this.data.idx, o.items.length - 1) });
    this.pickTicket(this.data.idx);
    if (o.status === 'unpaid') this.startTimer(); else this.stopTimer();
  },
  pickTicket(i) {
    const o = this.data.o;
    const it = o.items[i];
    const v = o.viewers[i] || o.viewers[0] || { name: '' };
    const no = o.id + '-' + (i + 1);
    this.setData({ idx: i, tk: { label: it.label, tier: it.tierName, viewer: v.name, mask: v.mask, no } });
    if (o.status === 'paid' || o.status === 'refunded') wx.nextTick(() => this.drawCode(no, o.status !== 'paid'));
  },
  drawCode(no, dim) {
    wx.createSelectorQuery().select('#qr').fields({ node: true, size: true }).exec((res) => {
      const c = res && res[0] && res[0].node;
      if (c) code.draw(c, QR, no, dim);
    });
  },
  prev() { if (this.data.idx > 0) this.pickTicket(this.data.idx - 1); },
  next() { if (this.data.idx < this.data.o.items.length - 1) this.pickTicket(this.data.idx + 1); },
  startTimer() {
    this.stopTimer();
    const step = () => {
      const ms = this.data.o.expireAt - Date.now();
      if (ms <= 0) { this.stopTimer(); this.load(); return; }
      const c = F.countdown(ms);
      this.setData({ left: c.m + ':' + c.s });
    };
    step();
    this.timer = setInterval(step, 1000);
  },
  stopTimer() { if (this.timer) { clearInterval(this.timer); this.timer = null; } },
  // 出示调亮：记下原亮度 → 调到最亮并常亮；关闭或离开页面时恢复
  toggleBright() { if (!this.data.o) return;
    if (this.data.bright) { this.restore(); return; }
    wx.getScreenBrightness({
      success: (r) => { this.oldBright = r.value; },
      complete: () => {
        wx.setScreenBrightness({ value: 1 });
        wx.setKeepScreenOn({ keepScreenOn: true });
        this.setData({ bright: true });
        wx.showToast({ title: '已调至最亮，请向检票员出示', icon: 'none' });
      }
    });
  },
  restore() {
    if (!this.data.bright) return;
    wx.setScreenBrightness({ value: typeof this.oldBright === 'number' ? this.oldBright : 0.5 });
    wx.setKeepScreenOn({ keepScreenOn: false });
    this.setData({ bright: false });
  },
  pay() { if (!this.data.o) return;
    payment.pay(this.data.o, (ok, o) => {
      if (ok) wx.redirectTo({ url: '/pages/result/result?id=' + o.id });
    });
  },
  cancel() { if (!this.data.o) return;
    wx.showModal({
      title: '取消订单', content: '取消后座位将释放给其他观众', confirmText: '取消订单', cancelText: '再想想', confirmColor: '#ff3b5c',
      success: (r) => {
        if (!r.confirm) return;
        const o = this.data.o; o.status = 'cancelled'; o.cancelReason = '用户取消';
        store.upsertOrder(o); this.load();
      }
    });
  },
  refund() { if (!this.data.o) return;
    const o = this.data.o;
    if (!o.refundable) {
      wx.showModal({ title: '无法退票', content: '本场演出主办方规定不支持退票，可在演出前转赠给好友使用。', showCancel: false });
      return;
    }
    const fee = Math.round(o.amount * 0.2);
    wx.showModal({
      title: '申请退票', content: '退票将收取票面 20% 手续费 ¥' + fee + '，预计退回 ¥' + (o.amount - fee) + '，1~3 个工作日原路到账。',
      confirmText: '确认退票', confirmColor: '#ff3b5c',
      success: (r) => {
        if (!r.confirm) return;
        wx.showLoading({ title: '提交中' });
        setTimeout(() => {
          wx.hideLoading();
          o.status = 'refunded'; o.refundAmount = o.amount - fee; o.refundAt = Date.now();
          store.upsertOrder(o); this.load();
          wx.showToast({ title: '退票成功', icon: 'success' });
        }, 500);
      }
    });
  },
  service() {
    if (!this.data.o) return;
    wx.showActionSheet({
      itemList: ['拨打客服 400-820-6688', '复制订单号'],
      success: (r) => {
        if (r.tapIndex === 0) wx.makePhoneCall({ phoneNumber: '4008206688', fail: () => wx.setClipboardData({ data: '400-820-6688' }) });
        else wx.setClipboardData({ data: this.data.o.id });
      }
    });
  },
  toShow() { if (!this.data.o) return; wx.navigateTo({ url: '/pages/detail/detail?id=' + this.data.o.showId }); }
});
