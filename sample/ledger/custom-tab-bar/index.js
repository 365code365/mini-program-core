// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath 图片（只画文字首字），这里用 image 自己画
Component({
  data: {
    selected: 0,
    color: '#8a8d96',
    selectedColor: '#f57c34',
    list: [
      { pagePath: 'pages/index/index', text: '明细', icon: '/images/tab/home-off.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: 'pages/stats/stats', text: '统计', icon: '/images/tab/chart-off.png', iconOn: '/images/tab/chart-on.png' },
      { pagePath: 'pages/mine/mine', text: '我的', icon: '/images/tab/mine-off.png', iconOn: '/images/tab/mine-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const ds = e.currentTarget.dataset
      this.setData({ selected: ds.index })
      wx.switchTab({ url: '/' + ds.path })
    }
  }
})
