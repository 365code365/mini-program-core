// 半星评分：每颗星分左右两半，点左半 = x.5 分，点右半 = 整分。
// properties：value（0~5，步长 0.5）、size（px）、readonly、showText；改分时 triggerEvent('change', {value})
const LABELS = ['', '很差', '较差', '一般', '满意', '非常满意'];

Component({
  properties: {
    value: { type: Number, value: 0 },
    size: { type: Number, value: 24 },
    gap: { type: Number, value: 6 },
    readonly: { type: Boolean, value: false },
    showText: { type: Boolean, value: true }
  },
  data: { stars: [0, 0, 0, 0, 0], text: '' },
  observers: {
    value(v) { this.render(v); }
  },
  lifetimes: {
    attached() { this.render(this.data.value); }
  },
  methods: {
    render(v) {
      v = Math.max(0, Math.min(5, Math.round((Number(v) || 0) * 2) / 2));
      const stars = [0, 1, 2, 3, 4].map(i => (v >= i + 1 ? 1 : v >= i + 0.5 ? 0.5 : 0));
      this.setData({ stars, text: v ? v.toFixed(1) + ' 分 · ' + LABELS[Math.ceil(v)] : '点击星星评分' });
    },
    pick(e) {
      const v = Number(e.currentTarget.dataset.v) / 2;
      this.render(v);
      this.triggerEvent('change', { value: v });
    }
  }
});
