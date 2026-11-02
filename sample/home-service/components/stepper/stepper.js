// 数量 / 时长步进器
Component({
  properties: {
    value: { type: Number, value: 1 },
    min: { type: Number, value: 1 },
    max: { type: Number, value: 99 },
    unit: { type: String, value: '' }
  },
  methods: {
    change(e) {
      const d = +e.currentTarget.dataset.d;
      const v = this.data.value + d;
      if (v < this.data.min) { wx.showToast({ title: '最少 ' + this.data.min + (this.data.unit || ''), icon: 'none' }); return; }
      if (v > this.data.max) { wx.showToast({ title: '最多 ' + this.data.max + (this.data.unit || ''), icon: 'none' }); return; }
      this.setData({ value: v });
      this.triggerEvent('change', { value: v });
    }
  }
});
