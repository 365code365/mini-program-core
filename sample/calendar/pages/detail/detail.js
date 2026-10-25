const D = require('../../utils/date.js')
const S = require('../../utils/store.js')

Page({
  data: { id: 0, ev: null, missing: false, info: null },
  onLoad(query) {
    this.setData({ id: Number(query.id) })
  },
  onShow() {
    // 编辑完回来要刷新
    const ev = S.getEvent(this.data.id)
    if (!ev) return this.setData({ ev: null, missing: true })
    this.setData({
      ev,
      missing: false,
      info: {
        dateLabel: D.longLabel(ev.date),
        time: S.timeText(ev),
        repeat: S.REPEATS[ev.repeat],
        remind: S.REMINDS[ev.remind],
        color: S.COLORS[ev.color].value,
        colorName: S.COLORS[ev.color].name
      }
    })
  },
  onEdit() {
    wx.navigateTo({ url: '/pages/edit/edit?id=' + this.data.id })
  },
  onDelete() {
    const ev = this.data.ev
    wx.showModal({
      title: '删除日程',
      content: ev.repeat ? '「' + ev.title + '」是重复日程，将删除全部重复' : '确定删除「' + ev.title + '」？',
      confirmText: '删除',
      confirmColor: '#f2453d',
      success: res => {
        if (!res.confirm) return
        S.saveEvents(S.loadEvents().filter(i => i.id !== ev.id))
        wx.showToast({ title: '已删除', icon: 'success' })
        if (getCurrentPages().length > 1) wx.navigateBack()
        else wx.reLaunch({ url: '/pages/month/month' })
      }
    })
  }
})
