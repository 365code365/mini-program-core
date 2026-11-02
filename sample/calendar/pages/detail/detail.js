const D = require('../../utils/date.js')
const S = require('../../utils/store.js')
const R = require('../../utils/remind.js')
// 系统日历的重复类型（addPhoneRepeatCalendar 的 repeatInterval），下标与 S.REPEATS 对应
const PHONE_REPEAT = ['', 'day', 'week', 'month']
Page({
  data: { id: 0, ev: null, missing: false, info: null },
  onLoad(query) {
    this.setData({ id: Number(query.id) })
  },
  onShow() {
    // 编辑完回来要刷新
    const ev = S.getEvent(this.data.id)
    if (!ev) return this.setData({ ev: null, missing: true })
    // 重复日程：从今天起最近一次发生（今天本身也算）
    const today = S.today()
    const next = ev.repeat ? R.nextOn(ev, today) : ''
    this.setData({
      ev,
      missing: false,
      info: {
        dateLabel: D.longLabel(ev.date),
        time: S.timeText(ev),
        repeat: S.REPEATS[ev.repeat],
        remind: S.REMINDS[ev.remind],
        color: S.COLORS[ev.color].value,
        colorName: S.COLORS[ev.color].name,
        next: next ? (next === today ? '今天' : D.longLabel(next)) : ''
      }
    })
  },
  onEdit() {
    wx.navigateTo({ url: '/pages/edit/edit?id=' + this.data.id })
  },
  onBackMonth() {
    wx.reLaunch({ url: '/pages/month/month' })
  },
  // 以这条为模板新建（编辑页按新建处理，保存后得到新 id）
  onDuplicate() {
    wx.navigateTo({ url: '/pages/edit/edit?copy=' + this.data.id })
  },
  text() {
    const ev = this.data.ev
    const i = this.data.info
    return ev.title + '\n' + i.dateLabel + ' ' + i.time + (ev.repeat ? '（' + i.repeat + '）' : '') + (ev.note ? '\n' + ev.note : '')
  },
  onCopyText() {
    wx.setClipboardData({ data: this.text() })
  },
  // 写进系统日历：重复日程用 addPhoneRepeatCalendar。宿主没接（桌面端）走 fail：改为复制文字，让用户手动粘贴
  onPhoneCalendar() {
    const ev = this.data.ev
    const start = D.parse(ev.date)
    if (!ev.allDay) {
      const p = ev.start.split(':')
      start.setHours(Number(p[0]), Number(p[1]))
    }
    const end = D.parse(ev.date)
    if (!ev.allDay) {
      const p = ev.end.split(':')
      end.setHours(Number(p[0]), Number(p[1]))
    }
    const off = R.OFFSETS[ev.remind]
    const opts = {
      title: ev.title,
      startTime: Math.floor(start.getTime() / 1000),
      endTime: Math.floor(end.getTime() / 1000),
      allDay: ev.allDay,
      description: ev.note || '',
      alarm: off !== null,
      alarmOffset: off === null ? 0 : off * 60,
      success: () => wx.showToast({ title: '已添加到日历', icon: 'success' }),
      fail: () => {
        wx.showModal({
          title: '无法写入系统日历',
          content: '当前环境不支持访问手机日历，可以复制日程文字后手动添加。',
          confirmText: '复制文字',
          success: res => { if (res.confirm) this.onCopyText() }
        })
      }
    }
    if (ev.repeat) {
      opts.repeatInterval = PHONE_REPEAT[ev.repeat]
      wx.addPhoneRepeatCalendar(opts)
    } else {
      wx.addPhoneCalendar(opts)
    }
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
