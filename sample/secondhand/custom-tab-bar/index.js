// 自定义 tabBar：引擎原生 tabBar 不画 iconPath，用图片图标自己画；「消息」上的未读红点也画在这里，
// 由各 tab 页 onShow 里 getTabBar().setData({ selected, unread }) 推过来（代替 wx.setTabBarBadge）。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，两个都要写
Component({
  data: {
    selected: 0,
    unread: 0,
    color: '#9aa0a8',
    selectedColor: '#1f2329',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '首页', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/publish/publish', path: 'pages/publish/publish', text: '发闲置', icon: '/images/tab/publish.png', iconOn: '/images/tab/publish-on.png' },
      { pagePath: '/pages/messages/messages', path: 'pages/messages/messages', text: '消息', icon: '/images/tab/msg.png', iconOn: '/images/tab/msg-on.png', badge: true },
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
