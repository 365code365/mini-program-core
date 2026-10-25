const S = require('../../utils/store.js')

Page({
  data: {
    sizeNames: ['3×3（入门）', '4×4（经典）', '5×5（休闲）'],
    sizeIndex: 1,
    anim: true,
    theme: 'classic',
    themes: S.THEMES
  },

  onLoad() {
    const s = S.loadSettings()
    this.setData({ sizeIndex: S.SIZES.indexOf(s.size), anim: s.anim, theme: s.theme })
  },

  save() {
    S.saveSettings({ size: S.SIZES[this.data.sizeIndex], anim: this.data.anim, theme: this.data.theme })
  },

  // picker 的 e.detail.value 是下标（number）
  onSize(e) {
    const i = Number(e.detail.value)
    if (i === this.data.sizeIndex) return
    this.setData({ sizeIndex: i })
    this.save()
    wx.showToast({ title: '回到游戏将以 ' + S.SIZES[i] + '×' + S.SIZES[i] + ' 重开', icon: 'none' })
  },

  onAnim(e) {
    this.setData({ anim: e.detail.value })
    this.save()
  },

  onTheme(e) {
    this.setData({ theme: e.detail.value })
    this.save()
  },

  onReset() {
    wx.showModal({
      title: '恢复默认',
      content: '棋盘 4×4、开启动画、经典暖棕主题',
      success: r => {
        if (!r.confirm) return
        this.setData({ sizeIndex: 1, anim: true, theme: 'classic' })
        this.save()
      }
    })
  }
})
