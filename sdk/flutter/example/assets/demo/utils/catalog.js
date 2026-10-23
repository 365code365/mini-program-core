// 演示页目录：首页 / 组件 / 接口 三个 tab 都从这里取（演示 CommonJS 的 require 与 module.exports）
const groups = [
  {
    key: 'comp',
    title: '内置组件',
    items: [
      { name: '基础内容', desc: 'view · text · rich-text · icon · progress', url: '/pages/basic/basic', icon: '基', color: '#07c160' },
      { name: '表单', desc: 'button · input · textarea · switch · checkbox · radio · slider · form', url: '/pages/form/form', icon: '表', color: '#10aeff' },
      { name: '选择器', desc: 'picker 五种 mode · columnchange · cancel', url: '/pages/picker/picker', icon: '选', color: '#fa9d3b' },
      { name: '轮播', desc: 'swiper 自动播放 · 衔接 · 纵向 · 多项 · 边距 · 受控', url: '/pages/swiper/swiper', icon: '轮', color: '#576b95' },
      { name: '滚动容器', desc: 'scroll-view 横纵 · 锚点 · 触底 · 自定义下拉刷新', url: '/pages/scroll/scroll', icon: '滚', color: '#ff6a00' },
      { name: '媒体', desc: 'image 裁剪模式 · GIF · 事件 · 预览 · 宿主承载的 video', url: '/pages/media/media', icon: '媒', color: '#e64340' },
      { name: '画布', desc: 'canvas 旧接口 · type="2d" · drawImage', url: '/pages/canvas/canvas', icon: '画', color: '#8a5cf5' },
      { name: '自定义组件', desc: 'properties · observers · 具名 slot · behaviors · triggerEvent', url: '/pages/custom/custom', icon: '定', color: '#1aad19' },
      { name: 'Skyline 组件与动画', desc: 'grid-view · movable-view · 手势组件 · draggable-sheet · animate · worklet', url: '/pages/skyline/skyline', icon: 'S', color: '#07c160' },
      { name: '补齐的官方组件', desc: 'page-container · root-portal · match-media · audio · editor · 原生占位', url: '/pages/builtin/builtin', icon: '补', color: '#10aeff' }
    ]
  },
  {
    key: 'lang',
    title: '语法与样式',
    items: [
      { name: 'WXML 语法', desc: '条件 · 列表 · wx:key · template · import · include · wxs', url: '/pages/wxml/wxml', icon: 'W', color: '#576b95' },
      { name: 'WXSS 样式', desc: 'flex · 定位 · 渐变 · 阴影 · transform · 变量 · 选择器 · @media', url: '/pages/wxss/wxss', icon: 'S', color: '#10aeff' },
      { name: '事件系统', desc: '冒泡 · catch · 捕获 · 长按 · 触摸 · dataset · hover-class', url: '/pages/event/event', icon: 'E', color: '#fa9d3b' },
      { name: '动画', desc: '@keyframes · 按压过渡 · wx.createAnimation', url: '/pages/animation/animation', icon: 'A', color: '#e64340' }
    ]
  },
  {
    key: 'api',
    title: '常用接口',
    items: [
      { name: '界面反馈', desc: 'Toast · Loading · Modal · ActionSheet · 导航栏', url: '/pages/feedback/feedback', icon: '界', color: '#07c160' },
      { name: '路由与生命周期', desc: 'navigateTo · redirectTo · reLaunch · switchTab · 页面栈', url: '/pages/route/route', icon: '路', color: '#576b95' },
      { name: '数据缓存', desc: 'Storage 同步 / 异步 · 落盘，重开还在', url: '/pages/storage/storage', icon: '存', color: '#fa9d3b' },
      { name: '网络', desc: 'request · abort · downloadFile · uploadFile', url: '/pages/network/network', icon: '网', color: '#10aeff' },
      { name: '节点与滚动', desc: 'SelectorQuery · IntersectionObserver · pageScrollTo · 触底', url: '/pages/query/query', icon: '查', color: '#8a5cf5' },
      { name: '补齐的接口', desc: 'EventChannel · Worker · UDP / TCP · WebAudio · 批量存储 · 媒体查询', url: '/pages/newapi/newapi', icon: '新', color: '#e64340' }
    ]
  },
  {
    key: 'page',
    title: '页面形态',
    items: [
      { name: '模拟 HTTP · 信息流', desc: 'Promise 封装 · 骨架屏 · 触底分页 · 点赞乐观更新 · 失败重试', url: '/pages/http/http', icon: '流', color: '#10aeff' },
      { name: '模拟 HTTP · 仪表盘', desc: '4 个接口并发 · 分块加载 / 失败 · 柱状图 · 真实网络对照', url: '/pages/dashboard/dashboard', icon: '仪', color: '#8a5cf5' },
      { name: '自定义导航栏', desc: 'navigationStyle: custom · 状态栏高度 · 安全区', url: '/pages/navcustom/navcustom', icon: '导', color: '#ff6a00' },
      { name: '综合场景（性能基准）', desc: '长列表 + CSS 动画 + 每秒 setData + 原生 video', url: '/pages/perf/perf', icon: '综', color: '#e64340' },
      { name: '详情页（静态）', desc: '带 query 打开 · 改标题 · 返回', url: '/pages/detail/detail?from=index', icon: '详', color: '#1aad19' }
    ]
  }
];

function pick(keys) {
  return groups.filter((g) => keys.indexOf(g.key) >= 0);
}

function count() {
  return groups.reduce((n, g) => n + g.items.length, 0);
}

module.exports = { groups, pick, count };
