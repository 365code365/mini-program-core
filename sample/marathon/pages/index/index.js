const { EVENT, GROUPS } = require('../../utils/data.js')
const C = require('../../utils/check.js')
const S = require('../../utils/store.js')

Page({
  data: { event: EVENT, groups: [], cd: { d: 0, h: 0, m: 0, s: 0, text: '' }, closed: false, myCount: 0 },
  onLoad(query) {
    // ?now=2027-03-15T08:00:00 把「现在」钉死：倒计时从这一刻起按真实时间往下走（测试 / 演示用）
    const injected = C.parseLocal(query.now ? decodeURIComponent(query.now) : '')
    this.base = injected ? injected.getTime() : Date.now()
    this.startedAt = Date.now()
    this.deadline = C.parseLocal(EVENT.deadline).getTime()
  },
  onShow() {
    this.refresh()
    this.tick()
    this.stop()
    this.timer = setInterval(() => this.tick(), 1000)
  },
  onHide() { this.stop() },
  onUnload() { this.stop() },
  stop() {
    if (this.timer) clearInterval(this.timer)
    this.timer = null
  },
  now() {
    return this.base + (Date.now() - this.startedAt)
  },
  tick() {
    const left = this.deadline - this.now()
    if (left <= 0) {
      this.stop()
      return this.setData({ closed: true, cd: C.split(0) })
    }
    this.setData({ closed: false, cd: C.split(left) })
  },
  refresh() {
    const groups = GROUPS.map((g, i) => {
      const taken = Math.min(S.takenOf(i), g.quota)
      return {
        name: g.name, short: g.short, dist: g.dist, fee: g.fee, quota: g.quota, taken,
        left: g.quota - taken, full: taken >= g.quota,
        percent: Math.round((taken / g.quota) * 100),
        age: g.minAge + '~' + g.maxAge + ' 周岁', cutoff: g.cutoff
      }
    })
    this.setData({ groups, myCount: S.entries().filter(e => e.status !== 'canceled').length })
  },
  onGroup(e) {
    const i = Number(e.currentTarget.dataset.i)
    if (this.data.closed) return wx.showToast({ title: '报名已截止', icon: 'none' })
    if (this.data.groups[i].full) return wx.showToast({ title: GROUPS[i].short + '名额已满', icon: 'none' })
    getApp().globalData.draft = null
    wx.navigateTo({ url: '/pages/signup/signup?group=' + i })
  },
  goMine() {
    wx.navigateTo({ url: '/pages/mine/mine' })
  }
})
