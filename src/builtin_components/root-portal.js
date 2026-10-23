// <root-portal>（官方 component/root-portal）：整个子树脱离页面、相对窗口定位（类似 position: fixed）。
// enable=false 时就在原处正常排版。virtualHost：宿主节点本身不占位置
Component({
  options: { virtualHost: true },
  properties: { enable: { type: Boolean, value: true } }
});
