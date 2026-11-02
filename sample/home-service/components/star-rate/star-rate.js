// 星级评分：value 变了由 observers 重算星星与文案；点星星 triggerEvent('change')，
// bubbles + composed：外层 view 上的 bind:change 也收得到（评价页用它统计「打分次数」）。
const WORDS = ['', '非常差', '差', '一般', '满意', '超赞'];

Component({
  properties: {
    value: { type: Number, value: 5 },
    size: { type: Number, value: 52 },
    readonly: { type: Boolean, value: false },
    showText: { type: Boolean, value: true }
  },
  data: { stars: [], word: '' },
  observers: {
    value(v) {
      const n = Math.max(0, Math.min(5, Math.round(v)));
      this.setData({ stars: [1, 2, 3, 4, 5].map((i) => i <= n), word: WORDS[n] });
    }
  },
  lifetimes: {
    attached() { if (!this.data.stars.length) this.setData({ stars: [1, 2, 3, 4, 5].map((i) => i <= this.data.value), word: WORDS[this.data.value] }); }
  },
  methods: {
    onTap(e) {
      if (this.data.readonly) return;
      const v = +e.currentTarget.dataset.v;
      this.setData({ value: v });
      this.triggerEvent('change', { value: v, word: WORDS[v] }, { bubbles: true, composed: true });
    }
  }
});
