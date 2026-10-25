// 日程与设置：落在本地 storage（cal_events / cal_settings），首次打开按「今天」铺几条示例日程
const D = require('./date.js')

const REPEATS = ['不重复', '每天', '每周', '每月']
const REMINDS = ['不提醒', '准时', '提前 5 分钟', '提前 15 分钟', '提前 1 小时', '提前 1 天']
const COLORS = [
  { name: '珊瑚', value: '#ff6b5b' },
  { name: '天蓝', value: '#3a86ff' },
  { name: '薄荷', value: '#19b37d' },
  { name: '琥珀', value: '#ffa41b' },
  { name: '葡萄', value: '#8e5cf7' }
]

// 「今天」：月视图带 ?today=2026-03-15 进来时用注入值（测试可确定），否则用系统时间
function today() {
  const g = getApp().globalData
  return g.today || D.fmtDate(new Date())
}

function seed() {
  const t = today()
  const ym = t.slice(0, 7)
  // 每周例会：从今天往前推到最近的周一，再往前两周起
  const mon = D.addDays(t, -((D.weekday(t) + 6) % 7) - 14)
  // 房租：两个月前的 5 号起每月一次
  const p = D.addMonths(Number(ym.slice(0, 4)), Number(ym.slice(5)), -2)
  const rent = D.fmt(p.year, p.month, 5)
  return [
    { id: 1, title: '产品周会', allDay: false, start: '10:00', end: '11:00', date: mon, repeat: 2, remind: 3, color: 1, note: '3 楼大会议室' },
    { id: 2, title: '交房租', allDay: true, start: '09:00', end: '10:00', date: rent, repeat: 3, remind: 5, color: 3, note: '' },
    { id: 3, title: '牙医复诊', allDay: false, start: '14:00', end: '15:00', date: t, repeat: 0, remind: 4, color: 0, note: '瑞尔齿科 · 滨江店' },
    { id: 4, title: '项目评审', allDay: false, start: '09:30', end: '11:30', date: t, repeat: 0, remind: 2, color: 4, note: '' },
    { id: 5, title: '妈妈生日', allDay: true, start: '09:00', end: '10:00', date: D.addDays(t, 4), repeat: 0, remind: 5, color: 0, note: '订芒果千层' },
    { id: 6, title: '读书会', allDay: false, start: '19:30', end: '21:00', date: D.addDays(t, 9), repeat: 0, remind: 1, color: 2, note: '《置身事内》' }
  ]
}

function loadEvents() {
  let list = wx.getStorageSync('cal_events')
  if (!Array.isArray(list)) {
    list = seed()
    wx.setStorageSync('cal_events', list)
  }
  return list
}
function saveEvents(list) {
  wx.setStorageSync('cal_events', list)
}
function getEvent(id) {
  return loadEvents().find(e => e.id === Number(id)) || null
}

const DEFAULT_SETTINGS = { weekStart: 1, defaultRemind: 1 }
function loadSettings() {
  return Object.assign({}, DEFAULT_SETTINGS, wx.getStorageSync('cal_settings') || {})
}
function saveSettings(s) {
  wx.setStorageSync('cal_settings', s)
}

// 重复规则展开：某条日程在 date 这天是否发生
function occursOn(ev, date) {
  if (date < ev.date) return false
  switch (ev.repeat) {
    case 1: return true
    case 2: return D.weekday(date) === D.weekday(ev.date)
    case 3: {
      // 每月：同一号；31 号的日程在小月落到月末
      const day = Number(ev.date.slice(8))
      const y = Number(date.slice(0, 4))
      const m = Number(date.slice(5, 7))
      return Number(date.slice(8)) === Math.min(day, D.daysInMonth(y, m))
    }
    default: return date === ev.date
  }
}
function timeText(ev) {
  return ev.allDay ? '全天' : ev.start + ' - ' + ev.end
}
// 某天的日程（已按时间排好，全天的在前）
function eventsOn(list, date) {
  return list
    .filter(ev => occursOn(ev, date))
    .sort((a, b) => (a.allDay === b.allDay ? (a.start < b.start ? -1 : a.start > b.start ? 1 : a.id - b.id) : a.allDay ? -1 : 1))
    .map(ev => ({ id: ev.id, title: ev.title, time: timeText(ev), color: COLORS[ev.color].value, repeat: ev.repeat ? REPEATS[ev.repeat] : '' }))
}

module.exports = { REPEATS, REMINDS, COLORS, today, loadEvents, saveEvents, getEvent, loadSettings, saveSettings, occursOn, eventsOn, timeText }
