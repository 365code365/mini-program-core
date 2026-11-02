// 模拟收银台：wx.requestPayment 不可用时由页面弹出。确认 → paid 事件；关闭 → cancel 事件。
// 支付方式用 radio-group；label 在引擎里不做表单关联，所以整行另绑 tap 直接选中。
Component({
  properties: {
    show: { type: Boolean, value: false },
    amount: { type: String, value: '0.00' },
    orderNo: { type: String, value: '' }
  },
  data: {
    way: 'wx',
    paying: false,
    ways: [
      { id: 'wx', name: '微信支付', sub: '零钱余额 ¥1,286.40', ic: '微', color: '#2aae67' },
      { id: 'card', name: '招商银行储蓄卡 (8826)', sub: '单笔限额 5 万元', ic: '招', color: '#c8152d' },
      { id: 'credit', name: '分付', sub: '本月免息，下月 10 日还', ic: '分', color: '#f0a020' }
    ]
  },
  methods: {
    noop() {},
    onWay(e) { this.setData({ way: e.detail.value }); },
    pickWay(e) { this.setData({ way: e.currentTarget.dataset.id }); },
    cancel() { if (!this.data.paying) this.triggerEvent('cancel'); },
    confirm() {
      if (this.data.paying) return;
      this.setData({ paying: true });
      wx.showLoading({ title: '支付中', mask: true });
      setTimeout(() => {
        wx.hideLoading();
        this.setData({ paying: false });
        this.triggerEvent('paid', { way: this.data.way });
      }, 600);
    }
  }
});
