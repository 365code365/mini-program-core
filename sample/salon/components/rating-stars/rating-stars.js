// 评分星级：value 支持半星（4.5），readonly=false 时点第 n 颗星把分数改成 n，并 triggerEvent('change', { value })
Component({
  properties: {
    value: { type: Number, value: 0 },
    max: { type: Number, value: 5 },
    size: { type: Number, value: 28 },      // 单颗星的边长（rpx）
    readonly: { type: Boolean, value: true },
    showScore: { type: Boolean, value: false }
  },
  data: { stars: [], scoreText: '' },
  observers: {
    // 两个字段任一变化都重算星星
    'value, max': function (value, max) {
      const stars = [];
      for (let i = 1; i <= max; i++) {
        stars.push(value >= i ? 'full' : value >= i - 0.5 ? 'half' : 'empty');
      }
      this.setData({ stars, scoreText: Number(value).toFixed(1) });
    }
  },
  lifetimes: {
    attached() {
      // observers 只在属性被设置时触发；使用方没传 value 时这里补一次，空星也要画出来
      if (!this.data.stars.length) this.setData({ stars: new Array(this.data.max).fill('empty'), scoreText: '0.0' });
    }
  },
  methods: {
    onTap(e) {
      if (this.data.readonly) return;
      const value = Number(e.currentTarget.dataset.i) + 1;
      this.setData({ value });
      this.triggerEvent('change', { value });
    }
  }
});
