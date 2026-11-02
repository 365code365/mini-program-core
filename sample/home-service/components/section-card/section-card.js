// 卡片容器：标题区（title + 右侧 slot "extra"）、正文（默认 slot）、底栏（slot "footer"）。
// 外部样式类 custom-class / title-class 让使用方改卡片外观；
// extClass 属性是给本引擎的兜底：引擎还没实现 externalClasses（使用方写的 custom-class="x" 不会映射，见报告），
// 页面样式会渗进组件，所以把同一个类名再从属性传一次就能生效。
Component({
  options: { multipleSlots: true, styleIsolation: 'apply-shared' },
  externalClasses: ['custom-class', 'title-class'],
  properties: {
    title: { type: String, value: '' },
    sub: { type: String, value: '' },
    extClass: { type: String, value: '' },
    footer: { type: Boolean, value: false }
  },
  methods: {
    onHead() { this.triggerEvent('headtap', { title: this.data.title }); }
  }
});
