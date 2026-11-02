// 自定义 tabBar：引擎原生 tabBar 不画 iconPath，用图片图标自己画。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，所以 wxml 两个都写。
Component({
  data: {
    selected: 0,
    color: '#9aa0a8',
    selectedColor: '#13a874',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '加油', icon: '/images/tab/station.png', iconOn: '/images/tab/station-on.png' },
      { pagePath: '/pages/trend/trend', path: 'pages/trend/trend', text: '油价', icon: '/images/tab/trend.png', iconOn: '/images/tab/trend-on.png' },
      { pagePath: '/pages/records/records', path: 'pages/records/records', text: '记录', icon: '/images/tab/record.png', iconOn: '/images/tab/record-on.png' },
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
