// 步进器：value / min / max 由外部给，点 ± 时 triggerEvent('change', { value, key })，外部自己 setData
Component({
  properties: {
    value: { type: Number, value: 0 },
    min: { type: Number, value: 0 },
    max: { type: Number, value: 9 },
    key: { type: String, value: '' }
  },
  methods: {
    onMinus() { this.emit(this.data.value - 1); },
    onPlus() {
      if (this.data.value >= this.data.max) { wx.showToast({ title: '单笔最多 ' + this.data.max + ' 张', icon: 'none' }); return; }
      this.emit(this.data.value + 1);
    },
    emit(v) {
      if (v < this.data.min || v > this.data.max) return;
      this.triggerEvent('change', { value: v, key: this.data.key });
    }
  }
});
