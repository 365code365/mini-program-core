// 表单校验 mixin：给 form-item 提供 rules / value 两个属性与 validate / clearError 方法。
// rules 形如 [{ required: true, msg }, { pattern: '^1[3-9]\\d{9}$', msg }, { min: 5, msg }, { max: 60, msg }]
module.exports = Behavior({
  properties: {
    name: { type: String, value: '' },
    value: { type: null, value: '' },
    rules: { type: Array, value: [] }
  },
  data: { error: '', touched: false },
  observers: {
    // 校验过一次之后，值一改就实时重新校验（第一次不打扰用户）
    value(v) { if (this.data.touched) this.check(v); }
  },
  methods: {
    check(v) {
      const s = Array.isArray(v) ? v.join('') : (v === null || v === undefined ? '' : String(v)).trim();
      let error = '';
      for (const r of this.data.rules) {
        if (r.required && !s) { error = r.msg; break; }
        if (!s) continue;
        if (r.pattern && !new RegExp(r.pattern).test(s)) { error = r.msg; break; }
        if (r.min && s.length < r.min) { error = r.msg; break; }
        if (r.max && s.length > r.max) { error = r.msg; break; }
      }
      if (error !== this.data.error) this.setData({ error });
      return !error;
    },
    // v 可选：本引擎里页面 setData 之后子组件的 properties 要到下一轮宏任务才更新（微信里是同步的，见报告），
    // 页面在同一个回调里改完值就校验时把当前值直接传进来
    validate(v) {
      if (!this.data.touched) this.setData({ touched: true });
      return this.check(v === undefined ? this.data.value : v);
    },
    clearError() { this.setData({ error: '', touched: false }); }
  }
});
