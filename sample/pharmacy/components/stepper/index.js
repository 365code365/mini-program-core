// 步进器：受控组件，value 由父页传入；点 ± 或改输入后 triggerEvent('change', {value})，父页决定写不写回。
// 到上下限时按钮置灰，并 triggerEvent('limit', {type: 'min' | 'max'}) 让父页提示（比如「最少 1 件」「限购 99 件」）
Component({
  properties: {
    value: { type: Number, value: 1 },
    min: { type: Number, value: 1 },
    max: { type: Number, value: 99 },
    size: { type: String, value: 'normal' }
  },
  data: { text: '1' },
  observers: {
    value(v) { this.setData({ text: String(v) }); }
  },
  methods: {
    emit(v) {
      const { min, max } = this.properties;
      const n = Math.max(min, Math.min(max, v));
      if (n !== v) this.triggerEvent('limit', { type: v < min ? 'min' : 'max' });
      if (n !== this.properties.value) this.triggerEvent('change', { value: n });
      else this.setData({ text: String(n) });
    },
    minus() { this.emit(this.properties.value - 1); },
    plus() { this.emit(this.properties.value + 1); },
    onBlur(e) {
      const n = parseInt(e.detail.value, 10);
      this.emit(isNaN(n) ? this.properties.min : n);
    }
  }
});
