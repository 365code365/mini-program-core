const app = getApp();
const HTML = '<h2 class="rt-title">富文本 HTML 字符串</h2>' +
  '<p>一段带 <b>加粗</b>、<span style="color:#e64340">红字</span> 和 <u>下划线</u> 的正文，<br>换行之后接着写。</p>' +
  '<ul><li>项目符号一</li><li>项目符号二</li></ul><ol start="3"><li>从 3 开始的编号</li><li>第四项</li></ol>' +
  '<blockquote>引用块按 WebKit 默认缩进</blockquote>' +
  '<p><img src="/images/photo.jpg" width="120" height="80"> 图片后面接着文字，到行尾自动折行。</p>' +
  '<iframe>白名单外的标签连子树一起丢掉，这行不会出现</iframe>';
const NODES = [{ name: 'div', attrs: { class: 'rt-box', style: 'padding:8px;border-radius:8px' }, children: [
  { type: 'text', text: '节点数组写法：class 匹配页面 WXSS，' },
  { name: 'span', attrs: { style: 'color:#07c160;font-weight:bold' }, children: [{ type: 'text', text: 'style 全量生效' }] }
] }];
Page({
  data: {
    long: '小程序引擎按 Skyline 的规则排版：多行文本超过行数时最后一行末尾显示省略号，这一段故意写得很长，用来看两行之后被截断的效果，以及 max-lines 与 overflow 四种收尾方式的区别。',
    html: HTML, nodes: NODES, overflow: 'ellipsis', modes: ['ellipsis', 'clip', 'fade', 'visible'], log: []
  },
  onMode(e) { this.setData({ overflow: e.currentTarget.dataset.m }); },
  onRichTap() { app.note(this, 'rich-text 内部节点不派事件，点击归 rich-text 自己'); }
});
