// behaviors：多个组件共用的 data 与方法（Behavior() 的字段会合并进使用它的组件）
module.exports = Behavior({
  data: {
    logs: []
  },
  methods: {
    log(text) {
      const logs = [text].concat(this.data.logs).slice(0, 4);
      this.setData({ logs });
    }
  }
});
