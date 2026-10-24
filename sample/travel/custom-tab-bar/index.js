Component({
  data: {
    selected: 0,
    color: '#a0a0a8',
    selectedColor: '#1e78ff',
    list: [
      { pagePath: 'pages/trips/trips', text: '行程', iconPath: '/images/tab/trips-off.png', selectedIconPath: '/images/tab/trips-on.png' },
      { pagePath: 'pages/explore/explore', text: '灵感', iconPath: '/images/tab/explore-off.png', selectedIconPath: '/images/tab/explore-on.png' },
      { pagePath: 'pages/mine/mine', text: '我的', iconPath: '/images/tab/mine-off.png', selectedIconPath: '/images/tab/mine-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const path = e.currentTarget.dataset.path;
      this.setData({ selected: Number(e.currentTarget.dataset.index) });
      wx.switchTab({ url: '/' + path });
    }
  }
});
