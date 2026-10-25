// 自定义 tabBar：图标用圆形字块（不依赖图片），选中态换主题色
Component({
  data: {
    selected: 0,
    color: '#9aa0ab',
    selectedColor: '#2b6de8',
    list: [
      { pagePath: '/pages/index/index', text: '首页', glyph: '车' },
      { pagePath: '/pages/orders/orders', text: '订单', glyph: '单' },
      { pagePath: '/pages/mine/mine', text: '我的', glyph: '我' }
    ]
  },
  methods: {
    switchTab(e) {
      const i = Number(e.currentTarget.dataset.index)
      wx.switchTab({ url: e.currentTarget.dataset.path })
      this.setData({ selected: i })
    }
  }
})
