// 模拟收银台：wx.requestPayment 不可用（桌面端 / 未接宿主支付）时由页面打开，用户再确认一次才算支付成功。
// 倒计时到点自动关闭并通知页面（订单超时）。
const order = require('../../utils/order');
const D = require('../../utils/date');

Component({
  properties: {
    show: { type: Boolean, value: false },
    orderId: { type: String, value: '' },
    amount: { type: Number, value: 0 },
    expireAt: { type: Number, value: 0 },
    reason: { type: String, value: '' }
  },
  data: {
    left: '', method: 'balance',
    methods: [
      { k: 'balance', name: '微信零钱', desc: '可用余额 ¥8,326.50' },
      { k: 'cmb', name: '招商银行储蓄卡', desc: '尾号 6217 · 单笔限额 ¥50,000' },
      { k: 'icbc', name: '工商银行信用卡', desc: '尾号 0385 · 可用额度 ¥21,400' }
    ]
  },
  observers: {
    show(v) { if (v) this.start(); else this.stop(); }
  },
  lifetimes: { detached() { this.stop(); } },
  methods: {
    start() {
      this.tick();
      this.stop();
      this._timer = setInterval(() => this.tick(), 1000);
    },
    stop() { if (this._timer) { clearInterval(this._timer); this._timer = null; } },
    tick() {
      const ms = this.properties.expireAt - Date.now();
      if (ms <= 0) {
        this.stop();
        this.triggerEvent('timeout');
        return;
      }
      const s = Math.floor(ms / 1000);
      this.setData({ left: D.pad(Math.floor(s / 60)) + ':' + D.pad(s % 60) });
    },
    onMethod(e) { this.setData({ method: e.detail.value }); },
    pay() {
      const m = this.data.methods.find(x => x.k === this.data.method);
      wx.showLoading({ title: '支付中', mask: true });
      setTimeout(() => {
        wx.hideLoading();
        wx.vibrateShort({ type: 'medium' });
        const o = order.markPaid(this.properties.orderId, m.name);
        this.stop();
        this.triggerEvent('paid', { id: o.id });
      }, 600);
    },
    close() {
      wx.showModal({
        title: '确定放弃支付？',
        content: '订单会为你保留到倒计时结束，超时后座位将释放，价格可能变动。',
        cancelText: '放弃', confirmText: '继续支付',
        success: (r) => { if (r.cancel) { this.stop(); this.triggerEvent('close'); } }
      });
    },
    noop() {}
  }
});
