const S = require('../../utils/store.js')

const COLORS = ['#ff6250', '#3a86ff', '#ff9f1a', '#19a974', '#8b5cf6']
const MAX_H = 260

Page({
  data: {
    unit: 0,
    week: [],
    sel: 6,
    gridLines: [1, 2, 3],
    weekPomos: 0,
    weekHours: 0,
    doneCount: 0,
    avgText: '',
    best: { label: '' },
    todayPomos: 0,
    goal: 8,
    goalPct: 0,
    goalDots: [],
    dist: []
  },

  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 2 })
    this.sessions = S.loadSessions()
    this.build()
  },

  build() {
    const sessions = this.sessions
    const unit = this.data.unit
    const days = []
    for (let i = -6; i <= 0; i++) {
      const date = S.dayOffset(i)
      const list = sessions.filter(s => s.date === date)
      const minutes = list.reduce((m, s) => m + s.minutes, 0)
      const d = new Date()
      d.setDate(d.getDate() + i)
      days.push({ date, count: list.length, minutes, isToday: i === 0, label: i === 0 ? '今天' : S.WEEK[d.getDay()] })
    }
    const vals = days.map(d => unit === 0 ? d.count : d.minutes)
    const max = Math.max.apply(null, vals.concat([1]))
    const week = days.map((d, k) => Object.assign(d, { value: vals[k], h: Math.max(8, Math.round(vals[k] / max * MAX_H)) }))
    const total = vals.reduce((a, b) => a + b, 0)
    const bestIdx = vals.indexOf(max)

    const weekSessions = sessions.filter(s => s.date >= days[0].date)
    const byTitle = {}
    weekSessions.forEach(s => {
      const t = byTitle[s.title] = byTitle[s.title] || { title: s.title, count: 0, minutes: 0 }
      t.count++
      t.minutes += s.minutes
    })
    const distRaw = Object.keys(byTitle).map(k => byTitle[k]).sort((a, b) => b.minutes - a.minutes).slice(0, 5)
    const topMin = distRaw.length ? distRaw[0].minutes : 1
    const st = S.loadSettings()
    const today = days[6]

    this.setData({
      week,
      weekPomos: days.reduce((a, d) => a + d.count, 0),
      weekHours: (days.reduce((a, d) => a + d.minutes, 0) / 60).toFixed(1),
      doneCount: S.loadTodos().filter(i => i.done).length,
      avgText: (total / 7).toFixed(1) + (unit === 0 ? ' 个' : ' 分钟'),
      best: { label: days[bestIdx].label + ' ' + max + (unit === 0 ? ' 个' : ' 分钟') },
      todayPomos: today.count,
      goal: st.goal,
      goalPct: Math.min(100, Math.round(today.count * 100 / st.goal)),
      goalDots: Array.from({ length: st.goal }, (_, i) => i),
      dist: distRaw.map((t, i) => Object.assign(t, { pct: Math.round(t.minutes * 100 / topMin), color: COLORS[i % COLORS.length] }))
    })
  },

  onUnit(e) {
    this.setData({ unit: Number(e.currentTarget.dataset.u) })
    this.build()
  },

  onBar(e) {
    this.setData({ sel: Number(e.currentTarget.dataset.i) })
  }
})
