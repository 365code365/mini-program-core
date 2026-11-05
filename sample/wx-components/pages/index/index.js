const PAGES = [
  { path: 'sticky', title: '吸顶', desc: 'position: sticky · sticky-section · sticky-header · stickontopchange' },
  { path: 'text', title: '文本与富文本', desc: 'line-clamp · max-lines · decode · space · rich-text HTML · span 混排' },
  { path: 'list', title: '列表构造器', desc: 'list-builder · grid-builder · itembuild / itemdispose · grid-view 瀑布流' },
  { path: 'form', title: '表单与输入', desc: 'focus · hold-keyboard · auto-height · radio 分组 · loading · progress active' },
  { path: 'picker', title: '选择器', desc: 'region 区划码 · custom-item · level · time start/end · picker-view 选中框' },
  { path: 'scroll', title: '滚动', desc: '下拉二级 · show-scrollbar · paging · min-drag-distance · ScrollViewContext · item-id' },
  { path: 'gesture', title: '手势与弹层', desc: 'worklet:ongesture · 手势协商 · draggable-sheet · page-container' },
  { path: 'media', title: '画布与媒体', desc: 'drawImage 9 参数 · roundRect · isPointInPath · SVG · fade-in · video 子节点' },
  { path: 'open', title: '开放能力', desc: 'button open-type · open-data · ad binderror · snapshot · navigator 分流' }
];
Page({
  data: { pages: PAGES },
  onShareAppMessage() { return { title: '组件实验室' }; }
});
