const S = require('../../utils/store.js')
// 补录专注：忘了开计时器时手动记一条。校验：不能记在将来、单条 5~120 分钟、一天合计不超过 16 小时
const DAY_MAX = 16 * 60
function nowHM() {
  const d = new Date()
  return S.pad(d.getHours()) + ':' + S.pad(d.getMinutes())
}
Page({
  data: {
    date: '',
    at: '',
    minutes: 25,
    taskNames: [],
    taskIds: [],
    taskIndex: 0,
    today: '',
    err: ''
  },
  onLoad() {
    const todos = S.loadTodos()
    const today = S.today()
    this.setData({
      today,
      date: today,
      at: nowHM(),
      minutes: S.loadSettings().focus,
      taskNames: ['自由专注'].concat(todos.map(i => i.title)),
      taskIds: [0].concat(todos.map(i => i.id))
    })
  },
  onDate(e) { this.setData({ date: e.detail.value, err: '' }) },
  onTime(e) { this.setData({ at: e.detail.value, err: '' }) },
  onMinutes(e) { this.setData({ minutes: e.detail.value, err: '' }) },
  onTask(e) { this.setData({ taskIndex: Number(e.detail.value) }) },
  validate() {
    const { date, at, minutes, today } = this.data
    if (date > today || (date === today && at > nowHM())) return '不能补录将来的专注'
    if (minutes < 5 || minutes > 120) return '单次专注 5~120 分钟'
    const used = S.loadSessions().filter(s => s.date === date).reduce((m, s) => m + s.minutes, 0)
    if (used + minutes > DAY_MAX) return '这天已专注 ' + used + ' 分钟，合计不能超过 16 小时'
    return ''
  },
  onSubmit() {
    const err = this.validate()
    if (err) {
      this.setData({ err })
      wx.showToast({ title: err, icon: 'none' })
      return
    }
    const taskId = this.data.taskIds[this.data.taskIndex]
    S.addSession({
      date: this.data.date, minutes: this.data.minutes, at: this.data.at,
      title: this.data.taskNames[this.data.taskIndex], taskId, manual: true
    })
    // 关联任务：一条记录算一个番茄（与计时器、记录页删除时减一是同一口径）
    if (taskId) {
      const list = S.loadTodos()
      const it = list.find(i => i.id === taskId)
      if (it) {
        it.pomos = (it.pomos || 0) + 1
        S.saveTodos(list)
      }
    }
    wx.showToast({ title: '已补录', icon: 'success' })
    setTimeout(() => wx.navigateBack(), 500)
  }
})
