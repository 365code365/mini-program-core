const SC = require('../../utils/scenes.js')
const D = require('../../utils/devices.js')

Page({
  data: {
    builtin: SC.BUILTIN,
    custom: [],
    running: '', // 正在执行的场景 id（执行中按钮置灰、防连点）
    last: '',
    lastResult: ''
  },

  onShow() {
    const names = {}
    D.load().forEach(d => { names[d.id] = d.name })
    const custom = SC.loadCustom().map(s => Object.assign({}, s, {
      desc: (s.action === 'on' ? '打开 ' : '关闭 ') + s.devices.map(id => names[id]).join('、')
    }))
    this.setData({ custom, last: wx.getStorageSync('sh_last_scene') || '' })
  },

  // 一键执行：showLoading → 模拟下发 800ms → 批量改设备 → toast
  onRun(e) {
    if (this.data.running) return
    const scene = SC.find(e.currentTarget.dataset.id)
    if (!scene) return
    this.setData({ running: scene.id })
    wx.showLoading({ title: '正在执行「' + scene.name + '」', mask: true })
    setTimeout(() => {
      const r = SC.run(scene)
      wx.hideLoading()
      const text = '已调整 ' + r.changed + ' 台设备' + (r.skipped ? '，' + r.skipped + ' 台离线已跳过' : '')
      this.setData({ running: '', last: scene.id, lastResult: scene.name + '：' + text })
      wx.showToast({ title: text, icon: r.skipped ? 'none' : 'success' })
    }, 800)
  },

  onAdd() {
    wx.navigateTo({ url: '/pages/scene-edit/scene-edit' })
  },

  // 长按自定义场景：删除
  onLong(e) {
    const id = e.currentTarget.dataset.id
    wx.showActionSheet({
      itemList: ['删除场景'],
      itemColor: '#e63946',
      success: res => {
        if (res.tapIndex !== 0) return
        SC.removeCustom(id)
        this.onShow()
        wx.showToast({ title: '已删除', icon: 'none' })
      }
    })
  }
})
