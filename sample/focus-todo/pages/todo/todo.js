const S = require('../../utils/store.js')

Page({
  data: {
    tab: 0,
    tabs: [
      { key: 'today', text: '今天', count: 0 },
      { key: 'plan', text: '计划', count: 0 },
      { key: 'done', text: '已完成', count: 0 }
    ],
    groups: [],
    newTitle: '',
    newPrio: 0,
    prioColor: S.PRIORITY[0].color,
    dateText: '',
    leftToday: 0,
    pct: 0
  },

  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 0 })
    this.refresh()
  },

  refresh() {
    const t = S.today()
    const all = S.loadTodos().map(i => Object.assign({}, i, { color: S.PRIORITY[i.priority].color }))
    const todayAll = all.filter(i => i.date === t)
    const todayLeft = todayAll.filter(i => !i.done)
    const plan = all.filter(i => !i.done && i.date > t)
    const done = all.filter(i => i.done)
    const byPrio = (a, b) => b.priority - a.priority || (a.time || '99').localeCompare(b.time || '99')

    let groups = []
    if (this.data.tab === 0) {
      if (todayLeft.length) groups = [{ key: 'today', title: '', items: todayLeft.sort(byPrio) }]
    } else if (this.data.tab === 1) {
      const map = {}
      plan.forEach(i => { (map[i.date] = map[i.date] || []).push(i) })
      groups = Object.keys(map).sort().map(d => ({ key: d, title: S.label(d), items: map[d].sort(byPrio) }))
    } else if (done.length) {
      groups = [{ key: 'done', title: '', items: done }]
    }

    const d = new Date()
    const pct = todayAll.length ? Math.round((todayAll.length - todayLeft.length) * 100 / todayAll.length) : 0
    this.setData({
      groups,
      'tabs[0].count': todayLeft.length,
      'tabs[1].count': plan.length,
      'tabs[2].count': done.length,
      leftToday: todayLeft.length,
      pct,
      dateText: (d.getMonth() + 1) + '月' + d.getDate() + '日 ' + S.WEEK[d.getDay()]
    })
  },

  onTab(e) {
    this.setData({ tab: Number(e.currentTarget.dataset.i) })
    this.refresh()
  },

  onToggle(e) {
    const id = Number(e.currentTarget.dataset.id)
    const list = S.loadTodos()
    const it = list.find(i => i.id === id)
    if (!it) return
    it.done = !it.done
    S.saveTodos(list)
    if (it.done) wx.showToast({ title: '完成一项 👏', icon: 'none' })
    this.refresh()
  },

  onOpen(e) {
    wx.navigateTo({ url: '/pages/edit/edit?id=' + e.currentTarget.dataset.id })
  },

  onFocus(e) {
    getApp().globalData.focusTaskId = Number(e.currentTarget.dataset.id)
    wx.switchTab({ url: '/pages/timer/timer' })
  },

  onCyclePrio() {
    const p = (this.data.newPrio + 1) % 4
    this.setData({ newPrio: p, prioColor: S.PRIORITY[p].color })
    wx.showToast({ title: '优先级：' + S.PRIORITY[p].text, icon: 'none', duration: 800 })
  },

  onNewInput(e) {
    this.setData({ newTitle: e.detail.value })
  },

  onAdd() {
    const title = (this.data.newTitle || '').trim()
    if (!title) return
    const list = S.loadTodos()
    const id = list.reduce((m, i) => Math.max(m, i.id), 0) + 1
    list.push({
      id, title, priority: this.data.newPrio,
      date: this.data.tab === 1 ? S.dayOffset(1) : S.today(),
      time: '', remind: false, note: '', done: false, pomos: 0
    })
    S.saveTodos(list)
    this.setData({ newTitle: '', newPrio: 0, prioColor: S.PRIORITY[0].color })
    this.refresh()
  }
})
