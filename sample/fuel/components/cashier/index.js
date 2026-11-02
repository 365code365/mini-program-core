// 模拟收银台：wx.requestPayment 不可用时由页面弹出。确认 → paid 事件（detail.way）；关闭 → cancel 事件。
// 支付方式用 radio-group；label 在引擎里不做表单关联，所以整行另绑 tap 直接选中。油卡余额不足时不能选。
Component({
  properties: {
    show: { type: Boolean, value: false },
    amount: { type: String, value: '0.00' },
    orderNo: { type: String, value: '' },
    cardBalance: { type: Number, value: -1 }
  },
  data: { way: 'wx', paying: false, ways: [] },
  observers: {
    'show, cardBalance, amount': function (show) {
      if (!show) return;
      const ways = [
        { id: 'wx', name: '微信支付', sub: '零钱余额 ¥2,318.60', ic: '微', color: '#13a874' },
        { id: 'bank', name: '建设银行储蓄卡 (6217)', sub: '单笔限额 5 万元', ic: '建', color: '#1d5bbf' }
      ];
      const bal = this.properties.cardBalance;
      if (bal >= 0) {
        const enough = bal >= parseFloat(this.properties.amount);
        ways.unshift({ id: 'card', name: '易加油电子油卡', sub: enough ? `余额 ¥${bal.toFixed(2)}` : `余额 ¥${bal.toFixed(2)}，不足本单`, ic: '卡', color: '#f2a900', off: !enough });
      }
      const first = ways.find((w) => !w.off);
      this.setData({ ways, way: this.data.ways.length && ways.some((w) => w.id === this.data.way && !w.off) ? this.data.way : first.id });
    }
  },
  methods: {
    noop() {},
    onWay(e) { this.pick(e.detail.value); },
    pickWay(e) { this.pick(e.currentTarget.dataset.id); },
    pick(id) {
      const w = this.data.ways.find((x) => x.id === id);
      if (!w || w.off) { wx.showToast({ title: '油卡余额不足，请先充值', icon: 'none' }); return; }
      this.setData({ way: id });
    },
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
