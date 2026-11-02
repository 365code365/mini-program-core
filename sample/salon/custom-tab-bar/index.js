// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath，所以用图片图标自己画。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，所以 wxml 两个都要写
Component({
  data: {
    selected: 0,
    color: '#9aa0a8',
    selectedColor: '#c0846a',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '首页', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/services/services', path: 'pages/services/services', text: '项目', icon: '/images/tab/service.png', iconOn: '/images/tab/service-on.png' },
      { pagePath: '/pages/stylists/stylists', path: 'pages/stylists/stylists', text: '发型师', icon: '/images/tab/stylist.png', iconOn: '/images/tab/stylist-on.png' },
      { pagePath: '/pages/mine/mine', path: 'pages/mine/mine', text: '我的', icon: '/images/tab/mine.png', iconOn: '/images/tab/mine-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const i = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[i].pagePath });
    }
  }
});
