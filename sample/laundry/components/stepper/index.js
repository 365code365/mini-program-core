// 步进器：properties value / max，加减时 triggerEvent('change', {value})。catchtap：不触发外层卡片的点击
Component({
  properties: {
    value: { type: Number, value: 0 },
    max: { type: Number, value: 99 }
  },
  methods: {
    plus() {
      if (this.data.value >= this.data.max) {
        wx.showToast({ title: '单项最多 ' + this.data.max + ' 件', icon: 'none' });
        return;
      }
      this.triggerEvent('change', { value: this.data.value + 1 });
    },
    minus() {
      if (this.data.value > 0) this.triggerEvent('change', { value: this.data.value - 1 });
    }
  }
});
