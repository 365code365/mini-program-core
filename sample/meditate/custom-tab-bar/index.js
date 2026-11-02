// 自定义 tabBar：引擎原生 tabBar 不画 iconPath，用图片图标自己画。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，所以 wxml 两个都要写
Component({
  data: {
    selected: 0,
    dark: false,
    color: '#9aa0b4',
    selectedColor: '#6c63d9',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '今日', icon: '/images/tab/lotus.png', iconOn: '/images/tab/lotus-on.png' },
      { pagePath: '/pages/category/category', path: 'pages/category/category', text: '课程', icon: '/images/tab/grid.png', iconOn: '/images/tab/grid-on.png' },
      { pagePath: '/pages/sleep/sleep', path: 'pages/sleep/sleep', text: '睡眠', icon: '/images/tab/moon.png', iconOn: '/images/tab/moon-on.png' },
      { pagePath: '/pages/mine/mine', path: 'pages/mine/mine', text: '我的', icon: '/images/tab/user.png', iconOn: '/images/tab/user-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const i = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[i].pagePath });
    }
  }
});
