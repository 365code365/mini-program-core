// 页内输入弹层：wx.showModal({ editable: true }) 拿不到 content 时的降级（见 utils/ask.js）。
Component({
  properties: {
    show: { type: Boolean, value: false },
    title: { type: String, value: '' },
    placeholder: { type: String, value: '' },
    confirmText: { type: String, value: '确定' },
    required: { type: Boolean, value: false }
  },
  data: { value: '', err: '' },
  observers: {
    show(v) { if (v) this.setData({ value: '', err: '' }); }
  },
  methods: {
    onInput(e) { this.setData({ value: e.detail.value, err: '' }); },
    cancel() { this.triggerEvent('cancel'); },
    confirm() {
      const v = this.data.value.trim();
      if (this.data.required && !v) return this.setData({ err: '请填写后再提交' });
      this.triggerEvent('confirm', { value: v });
    },
    noop() {}
  }
});
