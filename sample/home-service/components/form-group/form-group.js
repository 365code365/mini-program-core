// 表单分组：通过 relations 收集里面的 <form-item>，validate() 一次校验全部。
// 本引擎没实现 relations（linked 不触发、没有 getRelationNodes，见报告），
// 页面在提交前用 adopt(this.selectAllComponents('.fi')) 把 form-item 交进来，两条路收进同一个列表。
const ITEM = '../form-item/form-item';

Component({
  relations: {
    [ITEM]: {
      type: 'descendant',
      linked(target) { this.add(target); },
      unlinked(target) { this.items = (this.items || []).filter((t) => t !== target); }
    }
  },
  properties: { title: { type: String, value: '' } },
  data: { errors: 0 },
  lifetimes: {
    created() { this.items = []; }
  },
  methods: {
    add(t) { if (!this.items) this.items = []; if (this.items.indexOf(t) < 0) this.items.push(t); },
    adopt(list) { (list || []).forEach((t) => this.add(t)); return this.items.length; },
    // 表单项随 wx:if 增减时：没有 unlinked 回调可依赖，用页面当前的实例整体替换
    sync(list) { this.items = (list || []).slice(); return this.items.length; },
    count() {
      const rel = this.getRelationNodes ? this.getRelationNodes(ITEM) : [];
      return Math.max((rel || []).length, (this.items || []).length);
    },
    // 返回 { ok, values, first }：values 以 form-item 的 name 为键，first 是第一条错误
    validate() {
      const values = {};
      let first = '', errors = 0;
      (this.items || []).forEach((it) => {
        const ok = it.validate();
        values[it.data.name] = it.data.value;
        if (!ok) { errors++; if (!first) first = it.data.error; }
      });
      this.setData({ errors });
      return { ok: errors === 0, values, first };
    },
    reset() { (this.items || []).forEach((it) => it.clearError()); this.setData({ errors: 0 }); }
  }
});
