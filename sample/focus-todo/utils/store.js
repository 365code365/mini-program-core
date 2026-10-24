// 待办、专注记录、设置：全部落在本地 storage
const PRIORITY = [
  { text: '无', color: '#c4c9d2' },
  { text: '低', color: '#3a86ff' },
  { text: '中', color: '#ff9f1a' },
  { text: '高', color: '#f2453d' }
]

const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']

function pad(n) {
  return n < 10 ? '0' + n : '' + n
}

function fmt(d) {
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate())
}

function dayOffset(n) {
  const d = new Date()
  d.setDate(d.getDate() + n)
  return fmt(d)
}

function today() {
  return dayOffset(0)
}

// 「03-18 周一」这类展示
function label(dateStr) {
  if (dateStr === today()) return '今天'
  if (dateStr === dayOffset(1)) return '明天'
  const p = dateStr.split('-')
  const d = new Date(Number(p[0]), Number(p[1]) - 1, Number(p[2]))
  return p[1] + '-' + p[2] + ' ' + WEEK[d.getDay()]
}

function seedTodos() {
  return [
    { id: 1, title: '完成 Q1 产品复盘 PPT', priority: 3, date: dayOffset(0), time: '11:00', remind: true, note: '重点：留存曲线、付费转化、下季度 OKR 草案', done: false, pomos: 2 },
    { id: 2, title: '给设计组反馈首页改版稿', priority: 2, date: dayOffset(0), time: '14:30', remind: true, note: '', done: false, pomos: 0 },
    { id: 3, title: '背 30 个雅思单词', priority: 1, date: dayOffset(0), time: '21:00', remind: false, note: '剑桥真题 List 12', done: false, pomos: 1 },
    { id: 4, title: '取干洗的西装', priority: 0, date: dayOffset(0), time: '', remind: false, note: '', done: false, pomos: 0 },
    { id: 5, title: '晨跑 5 公里', priority: 1, date: dayOffset(0), time: '07:00', remind: false, note: '', done: true, pomos: 0 },
    { id: 6, title: '整理本周周报', priority: 2, date: dayOffset(0), time: '', remind: false, note: '', done: true, pomos: 1 },
    { id: 7, title: '预约牙医洗牙', priority: 2, date: dayOffset(1), time: '10:00', remind: true, note: '瑞尔齿科 · 滨江店', done: false, pomos: 0 },
    { id: 8, title: '读完《原则》第三章', priority: 1, date: dayOffset(2), time: '', remind: false, note: '', done: false, pomos: 0 },
    { id: 9, title: '妈妈生日 · 订蛋糕', priority: 3, date: dayOffset(4), time: '18:00', remind: true, note: '提前一天订，芒果千层', done: false, pomos: 0 }
  ]
}

function loadTodos() {
  let list = wx.getStorageSync('ft_todos')
  if (!list || !list.length) {
    list = seedTodos()
    wx.setStorageSync('ft_todos', list)
  }
  return list
}

function saveTodos(list) {
  wx.setStorageSync('ft_todos', list)
}

const DEFAULT_SETTINGS = { focus: 25, short: 5, long: 15, interval: 4, goal: 8, sound: true, autoBreak: false, daily: true }

function loadSettings() {
  return Object.assign({}, DEFAULT_SETTINGS, wx.getStorageSync('ft_settings') || {})
}

function saveSettings(s) {
  wx.setStorageSync('ft_settings', s)
}

// 专注记录：{ 'YYYY-MM-DD': 分钟 }，另存每条 session
function loadSessions() {
  let s = wx.getStorageSync('ft_sessions')
  if (!s || !s.length) {
    const seed = [[-6, 3], [-5, 5], [-4, 2], [-3, 6], [-2, 4], [-1, 7], [0, 2]]
    const titles = ['完成 Q1 产品复盘 PPT', '背 30 个雅思单词', '整理本周周报', '读完《原则》第三章']
    s = []
    seed.forEach(([d, n]) => {
      for (let i = 0; i < n; i++) {
        s.push({ date: dayOffset(d), minutes: 25, title: titles[(i + n) % titles.length], at: pad(9 + i) + ':' + (i % 2 ? '30' : '05') })
      }
    })
    wx.setStorageSync('ft_sessions', s)
  }
  return s
}

function addSession(session) {
  const s = loadSessions()
  s.push(session)
  wx.setStorageSync('ft_sessions', s)
}

module.exports = { PRIORITY, WEEK, pad, fmt, today, dayOffset, label, loadTodos, saveTodos, loadSettings, saveSettings, loadSessions, addSession }
