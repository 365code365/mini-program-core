// 自定义 tabBar：图标用圆形字块（不依赖图片），选中态换主题色
Component({
  data: {
    selected: 0,
    color: '#9aa0ab',
    selectedColor: '#ff7a2f',
    list: [
      { pagePath: '/pages/index/index', text: '菜谱', glyph: '厨' },
      { pagePath: '/pages/favs/favs', text: '收藏', glyph: '藏' },
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
