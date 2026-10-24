// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath，所以用 image 自己画图标
Component({
  data: {
    selected: 0,
    color: '#9aa0a6',
    selectedColor: '#5B4BF5',
    list: [
      { pagePath: '/pages/index/index', text: '首页', iconPath: '/images/tab-home.png', selectedIconPath: '/images/tab-home-on.png' },
      { pagePath: '/pages/rank/rank', text: '排行榜', iconPath: '/images/tab-rank.png', selectedIconPath: '/images/tab-rank-on.png' },
      { pagePath: '/pages/wrongbook/wrongbook', text: '错题本', iconPath: '/images/tab-book.png', selectedIconPath: '/images/tab-book-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const idx = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[idx].pagePath });
    }
  }
});
