const D = require('../../utils/data.js')
Page({
  data: {
    field: 'from',
    current: '',
    keyword: '',
    hot: D.HOT,
    groups: D.CITIES.map(g => ({ letter: g[0], cities: g[1] })),
    results: [],
    toView: ''
  },
  onLoad(q) {
    const field = q.field === 'to' ? 'to' : 'from'
    this.setData({ field, current: q.current || '' })
    wx.setNavigationBarTitle({ title: field === 'from' ? '选择出发城市' : '选择到达城市' })
  },
  onInput(e) {
    const kw = e.detail.value.trim()
    this.setData({ keyword: kw, results: kw ? D.ALL.filter(c => c.indexOf(kw) >= 0) : [] })
  },
  onLetter(e) {
    this.setData({ toView: 'g' + e.currentTarget.dataset.letter })
  },
  // 选中：结果交给 globalData，首页 onShow 取走
  onPick(e) {
    getApp().globalData.pickedCity = { field: this.data.field, name: e.currentTarget.dataset.name }
    wx.navigateBack()
  }
})
