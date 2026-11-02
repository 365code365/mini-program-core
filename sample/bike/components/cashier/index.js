// 模拟收银台：宿主没接微信支付时兜底，用户再点一次「确认支付」才算付款
Component({
  properties: {
    show: { type: Boolean, value: false },
    amount: { type: Number, value: 0 },
    title: { type: String, value: '' }
  },
  methods: {
    onConfirm() { this.triggerEvent('confirm'); },
    onCancel() { this.triggerEvent('cancel'); },
    noop() {}
  }
});
