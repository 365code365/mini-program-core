Component({
  data: {
    selected: 0,
    color: '#9aa0a6',
    selectedColor: '#FF6B3D',
    list: [
      { pagePath: '/pages/feed/feed', path: 'pages/feed/feed', text: '首页', iconPath: '/images/tab-home.png', selectedIconPath: '/images/tab-home-on.png' },
      { pagePath: '/pages/circles/circles', path: 'pages/circles/circles', text: '圈子', iconPath: '/images/tab-circles.png', selectedIconPath: '/images/tab-circles-on.png' },
      { pagePath: '/pages/mine/mine', path: 'pages/mine/mine', text: '我的', iconPath: '/images/tab-me.png', selectedIconPath: '/images/tab-me-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const idx = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[idx].pagePath });
    }
  }
});
