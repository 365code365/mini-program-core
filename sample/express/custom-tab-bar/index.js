Component({
  data: {
    selected: 0,
    color: '#9aa1ad',
    selectedColor: '#2f6bff',
    list: [
      { pagePath: '/pages/index/index', text: '查快递', iconPath: '/images/tab-home.png', selectedIconPath: '/images/tab-home-on.png' },
      { pagePath: '/pages/send/send', text: '寄快递', iconPath: '/images/tab-send.png', selectedIconPath: '/images/tab-send-on.png' },
      { pagePath: '/pages/mine/mine', text: '我的', iconPath: '/images/tab-mine.png', selectedIconPath: '/images/tab-mine-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const path = e.currentTarget.dataset.path
      wx.switchTab({ url: path })
      this.setData({ selected: Number(e.currentTarget.dataset.index) })
    }
  }
})
