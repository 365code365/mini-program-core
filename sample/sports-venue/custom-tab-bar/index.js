// 自定义 tabBar：引擎原生 tabBar 不画 iconPath，用图片图标自己画。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，所以 wxml 两个都要写
Component({
  data: {
    selected: 0,
    color: '#969aa4',
    selectedColor: '#13b37b',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '订场', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/teams/teams', path: 'pages/teams/teams', text: '约球', icon: '/images/tab/team.png', iconOn: '/images/tab/team-on.png' },
      { pagePath: '/pages/card/card', path: 'pages/card/card', text: '会员卡', icon: '/images/tab/card.png', iconOn: '/images/tab/card-on.png' },
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
