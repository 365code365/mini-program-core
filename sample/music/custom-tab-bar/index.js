// 自定义 tabBar：图标用圆形字块（不依赖图片），选中态换主题色
Component({
  data: {
    selected: 0,
    color: '#9aa0ab',
    selectedColor: '#ff3a5c',
    list: [
      { pagePath: '/pages/discover/discover', text: '发现', glyph: '发' },
      { pagePath: '/pages/library/library', text: '曲库', glyph: '曲' },
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
