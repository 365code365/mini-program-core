const S = require('../../utils/store.js')

// 不用 emoji 奖牌：仓库字体里没有彩色 emoji，会画成方框
const MEDALS = ['冠', '亚', '季']

Page({
  data: {
    list: [], // 前 10 名（按分数从高到低）
    filter: 0, // 0 全部 / 1 只看 4×4
    filters: ['全部尺寸', '只看 4×4']
  },

  onShow() {
    this.load()
  },

  load() {
    let all = S.loadHistory().slice()
    if (this.data.filter === 1) all = all.filter(r => r.size === 4)
    all.sort((a, b) => b.score - a.score)
    const list = all.slice(0, 10).map((r, i) => Object.assign({}, r, { rank: i + 1, medal: MEDALS[i] || '' }))
    this.setData({ list })
  },

  onFilter(e) {
    this.setData({ filter: Number(e.currentTarget.dataset.i) })
    this.load()
  },

  onClear() {
    if (!this.data.list.length) {
      wx.showToast({ title: '还没有成绩', icon: 'none' })
      return
    }
    wx.showModal({
      title: '清空排行榜',
      content: '所有历史成绩都会被删除，不能恢复',
      confirmText: '清空',
      confirmColor: '#e0574a',
      success: r => {
        if (!r.confirm) return
        S.clearHistory()
        this.load()
        wx.showToast({ title: '已清空', icon: 'success' })
      }
    })
  }
})
