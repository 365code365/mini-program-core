// 步骤条：steps 是标题数组，current 是当前步（0 起）。已完成的步可以点回去（向导里触发 jump 事件回退，表单数据不丢）。
Component({
  properties: {
    steps: { type: Array, value: [] },
    current: { type: Number, value: 0 }
  },
  methods: {
    onTap(e) {
      const i = e.currentTarget.dataset.i;
      if (i < this.data.current) this.triggerEvent('jump', { index: i });
    }
  }
});
