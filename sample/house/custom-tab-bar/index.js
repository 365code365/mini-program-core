Component({
  data: {
    selected: 0,
    color: '#9aa1ad',
    selectedColor: '#ff6a1f',
    list: [
      { pagePath: '/pages/index/index', text: '首页', iconPath: '/images/tab-home.png', selectedIconPath: '/images/tab-home-on.png' },
      { pagePath: '/pages/favorite/favorite', text: '收藏', iconPath: '/images/tab-fav.png', selectedIconPath: '/images/tab-fav-on.png' },
      { pagePath: '/pages/mine/mine', text: '我的', iconPath: '/images/tab-mine.png', selectedIconPath: '/images/tab-mine-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      wx.switchTab({ url: e.currentTarget.dataset.path })
      this.setData({ selected: Number(e.currentTarget.dataset.index) })
    }
  }
})
