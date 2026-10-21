// 具名 slot 要开 multipleSlots；properties 带类型，页面传字符串进来会按类型转换
Component({
  options: {
    multipleSlots: true
  },
  properties: {
    tag: { type: String, value: '' },
    accent: { type: String, value: '#07c160' }
  },
  data: {
    folded: false
  },
  methods: {
    toggle() {
      this.setData({ folded: !this.data.folded });
      this.triggerEvent('fold', { folded: this.data.folded });
    }
  }
});
