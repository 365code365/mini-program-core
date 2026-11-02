// 自定义 tabBar（引擎的原生 tabBar 不画图标）：节点同时写 data-index 与 data-path，引擎靠它直接切页
Component({
  data: {
    selected: 0,
    color: '#8a94a0',
    selectedColor: '#1c3d5a',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '地图', icon: '/images/tab/map.png', iconOn: '/images/tab/map-on.png' },
      { pagePath: '/pages/route/route', path: 'pages/route/route', text: '我的路线', icon: '/images/tab/route.png', iconOn: '/images/tab/route-on.png' },
      { pagePath: '/pages/mine/mine', path: 'pages/mine/mine', text: '我的', icon: '/images/tab/user.png', iconOn: '/images/tab/user-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      wx.switchTab({ url: this.data.list[e.currentTarget.dataset.index].pagePath });
    }
  }
});
