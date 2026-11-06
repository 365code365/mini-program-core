const PAGES = [
  { path: 'sticky', title: '吸顶', desc: 'position: sticky · sticky-section · sticky-header · stickontopchange' },
  { path: 'text', title: '文本与富文本', desc: 'line-clamp · max-lines · decode · space · rich-text HTML · span 混排' },
  { path: 'list', title: '列表构造器', desc: 'list-builder · grid-builder · itembuild / itemdispose · grid-view 瀑布流' },
  { path: 'longlist', title: '长列表改一行', desc: '300 行 wx:for · 只改一行 / 顶上计数 / 追加 50 行，看 setData 到画完的耗时' },
  { path: 'table', title: '横竖同时滚', desc: 'scroll-x + scroll-y 表格 · bindscroll 的 scrollLeft / scrollTop · 滚过之后点格子' },
  { path: 'form', title: '表单与输入', desc: 'focus · hold-keyboard · auto-height · radio 分组 · loading · progress active' },
  { path: 'picker', title: '选择器', desc: 'region 区划码 · custom-item · level · time start/end · picker-view 选中框' },
  { path: 'scroll', title: '滚动', desc: '下拉二级 · show-scrollbar · paging · min-drag-distance · ScrollViewContext · item-id' },
  { path: 'gesture', title: '手势与弹层', desc: 'worklet:ongesture · 手势协商 · draggable-sheet · page-container' },
  { path: 'media', title: '画布与媒体', desc: 'drawImage 9 参数 · roundRect · isPointInPath · SVG · fade-in · video 子节点' },
  { path: 'open', title: '开放能力', desc: 'button open-type · open-data · ad binderror · snapshot · navigator 分流' },
  // 下面几项是试验性的原生能力：画面由宿主的相机 / 播放器 / 编码器出，同层画进页面
  { path: 'scan', title: '二维码与条形码', desc: 'camera mode="scanCode" · bindscancode · wx.scanCode · 测试码（QR / EAN-13 / Code 128）', lab: true },
  { path: 'camera', title: '相机', desc: 'CameraContext 拍照 / 录像 / 变焦 · 前后置 · 闪光灯 · onCameraFrame', lab: true },
  { path: 'pusher', title: '直播推流', desc: 'live-pusher RTMP · 美颜 / 美白 / 滤镜 · 背景音 · 小窗 · snapshot', lab: true },
  { path: 'player', title: '直播拉流', desc: 'live-player HLS / mp4 · LivePlayerContext · 状态码 · 全屏 · 截图', lab: true },
  { path: 'voip', title: '实时音视频', desc: 'voip-room camera / video · joinVoIPChat（宿主接自己的通话后台）', lab: true }
];
Page({
  data: { pages: PAGES },
  onShareAppMessage() { return { title: '组件实验室' }; }
});
