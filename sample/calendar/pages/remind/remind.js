const D = require('../../utils/date.js')
const S = require('../../utils/store.js')
const R = require('../../utils/remind.js')
// 未来 7 天（含今天）设了提醒的日程，按提醒时刻排。「知道了」按「日程 id@日期」记在 cal_ack
const DAYS = 7
const ACK_KEY = 'cal_ack'
const SUB_KEY = 'cal_subscribed'
// 订阅消息模板（日程提醒）：桌面端没有宿主，调用会走 fail
const TMPL_ID = 'Kq3vH2f8T0dWm5yR7pLx1cNbZaE6uGjS9oIiQwYtVeA'
Page({
  data: { pending: [], handled: [], subscribed: false, range: '' },
  onShow() {
    this.refresh()
  },
  refresh() {
    const today = S.today()
    const events = S.loadEvents()
    const ack = wx.getStorageSync(ACK_KEY) || {}
    const now = R.nowAbs()
    const rows = []
    for (let i = 0; i < DAYS; i++) {
      const date = D.addDays(today, i)
      events.forEach(ev => {
        if (!S.occursOn(ev, date)) return
        const at = R.remindAt(ev, date)
        if (!at) return
        const key = ev.id + '@' + date
        rows.push({
          key,
          id: ev.id,
          title: ev.title,
          color: S.COLORS[ev.color].value,
          evTime: (i === 0 ? '今天' : i === 1 ? '明天' : D.label(date)) + ' ' + S.timeText(ev),
          remindText: (at.date === today ? '今天 ' : D.label(at.date) + ' ') + at.time + ' 提醒 · ' + S.REMINDS[ev.remind],
          abs: at.abs,
          due: at.abs <= now,
          acked: !!ack[key]
        })
      })
    }
    rows.sort((a, b) => a.abs - b.abs || a.id - b.id)
    this.setData({
      pending: rows.filter(r => !r.acked),
      handled: rows.filter(r => r.acked),
      subscribed: !!wx.getStorageSync(SUB_KEY),
      range: today.slice(5) + ' ~ ' + D.addDays(today, DAYS - 1).slice(5)
    })
  },
  setAck(key, on) {
    const ack = wx.getStorageSync(ACK_KEY) || {}
    if (on) ack[key] = true
    else delete ack[key]
    wx.setStorageSync(ACK_KEY, ack)
    this.refresh()
  },
  onAck(e) {
    this.setAck(e.currentTarget.dataset.key, true)
    wx.showToast({ title: '已标记', icon: 'none' })
  },
  onUndo(e) {
    this.setAck(e.currentTarget.dataset.key, false)
  },
  onAckAll() {
    if (!this.data.pending.length) return
    const ack = wx.getStorageSync(ACK_KEY) || {}
    this.data.pending.forEach(r => { ack[r.key] = true })
    wx.setStorageSync(ACK_KEY, ack)
    wx.showToast({ title: '全部标为已知晓', icon: 'none' })
    this.refresh()
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  },
  // 微信提醒走订阅消息；拿不到授权（桌面端 / 用户拒绝）时只在本页提醒
  onSubscribe() {
    if (this.data.subscribed) {
      wx.removeStorageSync(SUB_KEY)
      this.setData({ subscribed: false })
      wx.showToast({ title: '已关闭微信提醒', icon: 'none' })
      return
    }
    wx.requestSubscribeMessage({
      tmplIds: [TMPL_ID],
      success: res => {
        if (res[TMPL_ID] === 'accept') {
          wx.setStorageSync(SUB_KEY, true)
          this.setData({ subscribed: true })
          wx.showToast({ title: '已开启微信提醒', icon: 'success' })
        } else {
          wx.showToast({ title: '未授权，提醒只在本页显示', icon: 'none' })
        }
      },
      fail: () => {
        wx.showModal({
          title: '暂时无法开启',
          content: '当前环境不支持订阅消息，到点的提醒会显示在本页「待提醒」里',
          showCancel: false,
          confirmText: '知道了'
        })
      }
    })
  }
})
