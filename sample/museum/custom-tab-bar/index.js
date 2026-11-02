// 自定义 tabBar：引擎原生 tabBar 不画 iconPath，用图片图标自己画。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，所以 wxml 两个都要写
Component({
  data: {
    selected: 0,
    color: '#9aa0a8',
    selectedColor: '#a45c34',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '首页', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/halls/halls', path: 'pages/halls/halls', text: '展厅', icon: '/images/tab/hall.png', iconOn: '/images/tab/hall-on.png' },
      { pagePath: '/pages/calendar/calendar', path: 'pages/calendar/calendar', text: '日历', icon: '/images/tab/cal.png', iconOn: '/images/tab/cal-on.png' },
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
