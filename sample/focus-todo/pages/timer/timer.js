const S = require('../../utils/store.js')

const MODES = [
  { key: 'focus', text: '专注', color: '#ff6250', track: '#ffe4de', shadow: 'rgba(255,98,80,0.35)', state: '专注中' },
  { key: 'short', text: '短休息', color: '#19a974', track: '#d9f3e8', shadow: 'rgba(25,169,116,0.35)', state: '休息一下' },
  { key: 'long', text: '长休息', color: '#3a86ff', track: '#dde9ff', shadow: 'rgba(58,134,255,0.35)', state: '好好放松' }
]

Page({
  data: {
    modes: MODES,
    mode: 0,
    color: MODES[0].color,
    track: MODES[0].track,
    shadow: MODES[0].shadow,
    total: 1500,
    remain: 1500,
    running: false,
    timeText: '25:00',
    mainText: '开始',
    stateText: '准备开始',
    rightDeg: 0,
    leftDeg: 0,
    round: 0,
    roundDots: [0, 1, 2, 3],
    taskNames: ['不关联任务'],
    taskIds: [0],
    taskIndex: 0,
    todayPomos: 0,
    todayMinutes: 0,
    goal: 8
  },

  onLoad() {
    this.applyMode(0)
  },

  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 1 })
    const todos = S.loadTodos().filter(i => !i.done)
    const names = ['不关联任务'].concat(todos.map(i => i.title))
    const ids = [0].concat(todos.map(i => i.id))
    const g = getApp().globalData
    let taskIndex = this.data.taskIndex
    if (g.focusTaskId) {
      taskIndex = Math.max(0, ids.indexOf(g.focusTaskId))
      g.focusTaskId = null
    }
    const st = S.loadSettings()
    // 设置页改了时长：没在跑时立即生效
    if (!this.data.running && this.data.remain === this.data.total) {
      this.settings = st
      this.applyMode(this.data.mode)
    }
    this.settings = st
    this.setData({ taskNames: names, taskIds: ids, taskIndex, goal: st.goal, roundDots: Array.from({ length: st.interval }, (_, i) => i) })
    this.refreshToday()
  },

  onHide() {
    // 切走不停表：真机上番茄钟应在后台继续，这里保持 interval
  },

  onUnload() {
    this.stop()
  },

  refreshToday() {
    const t = S.today()
    const list = S.loadSessions().filter(s => s.date === t)
    this.setData({ todayPomos: list.length, todayMinutes: list.reduce((m, s) => m + s.minutes, 0) })
  },

  applyMode(i) {
    const st = this.settings || S.loadSettings()
    const mins = [st.focus, st.short, st.long][i]
    const m = MODES[i]
    this.setData({
      mode: i, color: m.color, track: m.track, shadow: m.shadow,
      total: mins * 60, remain: mins * 60, stateText: '准备开始'
    })
    this.render()
  },

  render() {
    const { remain, total } = this.data
    const p = total ? (total - remain) / total : 0
    const mm = Math.floor(remain / 60)
    const ss = remain % 60
    this.setData({
      mainText: this.data.running ? '暂停' : (remain < total ? '继续' : '开始'),
      timeText: S.pad(mm) + ':' + S.pad(ss),
      rightDeg: Math.min(p, 0.5) * 360,
      leftDeg: Math.max(p - 0.5, 0) * 360
    })
  },

  start() {
    if (this.timer) return
    this.setData({ running: true, mainText: '暂停', stateText: MODES[this.data.mode].state })
    this.timer = setInterval(() => this.tick(), 1000)
  },

  stop() {
    if (this.timer) clearInterval(this.timer)
    this.timer = null
    this.setData({ running: false })
    this.render()
  },

  tick() {
    const remain = this.data.remain - 1
    this.setData({ remain: Math.max(remain, 0) })
    this.render()
    if (remain <= 0) this.finish()
  },

  finish() {
    this.stop()
    const isFocus = this.data.mode === 0
    if (isFocus) {
      const d = new Date()
      const taskId = this.data.taskIds[this.data.taskIndex]
      S.addSession({
        date: S.today(), minutes: Math.round(this.data.total / 60),
        title: taskId ? this.data.taskNames[this.data.taskIndex] : '自由专注',
        at: S.pad(d.getHours()) + ':' + S.pad(d.getMinutes())
      })
      if (taskId) {
        const list = S.loadTodos()
        const it = list.find(i => i.id === taskId)
        if (it) { it.pomos = (it.pomos || 0) + 1; S.saveTodos(list) }
      }
      const round = this.data.round + 1
      this.setData({ round: round % this.settings.interval })
      this.refreshToday()
    }
    if (this.settings.sound) wx.vibrateLong({ fail() {} })
    wx.showModal({
      title: isFocus ? '完成一个番茄 🍅' : '休息结束',
      content: isFocus ? '干得漂亮！起来活动一下，喝口水吧。' : '准备好进入下一轮专注了吗？',
      confirmText: isFocus ? '开始休息' : '开始专注',
      confirmColor: '#ff6250',
      success: res => {
        const next = isFocus ? (this.data.round === 0 ? 2 : 1) : 0
        this.applyMode(next)
        if (res.confirm) this.start()
      }
    })
  },

  onToggle() {
    if (this.data.running) {
      this.stop()
      this.setData({ stateText: '已暂停' })
    } else {
      this.start()
    }
  },

  onReset() {
    this.stop()
    this.applyMode(this.data.mode)
  },

  onSkip() {
    if (this.data.remain === this.data.total && !this.data.running) {
      this.applyMode((this.data.mode + 1) % 3)
      return
    }
    wx.showModal({
      title: '提前结束？',
      content: '本轮进度不会计入统计',
      confirmColor: '#ff6250',
      success: res => {
        if (res.confirm) {
          this.stop()
          this.applyMode((this.data.mode + 1) % 3)
        }
      }
    })
  },

  onMode(e) {
    const i = Number(e.currentTarget.dataset.i)
    if (i === this.data.mode) return
    this.stop()
    this.applyMode(i)
  },

  onTask(e) {
    this.setData({ taskIndex: Number(e.detail.value) })
  }
})
