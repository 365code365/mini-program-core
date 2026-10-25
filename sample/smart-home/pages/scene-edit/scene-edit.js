const SC = require('../../utils/scenes.js')
const D = require('../../utils/devices.js')

Page({
  data: {
    name: '',
    action: 'on',
    selected: [], // 勾选的设备 id
    groups: [], // 按房间分组的在线设备：[{ room, list: [{ id, name, checked }] }]
    error: ''
  },

  onLoad() {
    this.build()
  },

  build() {
    const sel = this.data.selected
    const list = D.load().filter(d => d.online)
    this.setData({
      groups: D.ROOMS.map(r => ({
        room: r.name,
        list: list.filter(d => d.room === r.key).map(d => ({ id: d.id, name: d.name, checked: sel.indexOf(d.id) >= 0 }))
      })).filter(g => g.list.length)
    })
  },

  onName(e) {
    this.setData({ name: e.detail.value, error: '' })
  },

  onAction(e) {
    this.setData({ action: e.detail.value })
  },

  // 每个房间一个 checkbox-group：合并成一份勾选表
  onCheck(e) {
    const room = e.currentTarget.dataset.room
    const ids = this.data.groups.find(g => g.room === room).list.map(d => d.id)
    const keep = this.data.selected.filter(id => ids.indexOf(id) < 0)
    this.setData({ selected: keep.concat(e.detail.value), error: '' })
    this.build()
  },

  onSave() {
    const err = SC.validate(this.data.name, this.data.selected)
    if (err) {
      this.setData({ error: err })
      return
    }
    SC.addCustom(this.data.name, this.data.selected, this.data.action)
    wx.showToast({ title: '场景已保存', icon: 'success' })
    wx.navigateBack()
  }
})
