const W = require('../../utils/data.js')

Page({
  data: {
    navTop: 20, navH: 44, capsuleW: 96,
    city: null, bg: '', condName: '', temp: 0, feels: 0, hiT: 0, loT: 0, unit: 'C',
    hours: [], days: [], aqi: null, life: [], mine: [], ci: 0, updated: ''
  },
  onLoad() {
    // 自定义导航栏：顶部让出状态栏，右侧给胶囊让位
    let navTop = 20, navH = 44, capsuleW = 96
    try {
      const win = wx.getWindowInfo ? wx.getWindowInfo() : wx.getSystemInfoSync()
      const mb = wx.getMenuButtonBoundingClientRect()
      navTop = win.statusBarHeight || 20
      if (mb && mb.height) {
        navH = (mb.top - navTop) * 2 + mb.height
        capsuleW = win.windowWidth - mb.left + 8
      }
    } catch (e) {}
    this.setData({ navTop, navH, capsuleW })
  },
  onShow() { this.render() },
  onPullDownRefresh() { this.render(); wx.stopPullDownRefresh() },
  render() {
    const id = W.current()
    const c = W.byId(id)
    const now = new Date()
    const u = W.unit()
    const t = W.tempAt(c, now.getHours())
    const hours = W.hourly(c, now).map(h => Object.assign(h, { tt: W.fmtT(h.t, u) }))
    const days = W.daily(c, now)
    // 温度条：整周的最低~最高映射到 0~100%
    let min = 99, max = -99
    days.forEach(d => { min = Math.min(min, d.lo); max = Math.max(max, d.hi) })
    const span = Math.max(1, max - min)
    days.forEach(d => {
      d.left = Math.round((d.lo - min) / span * 100)
      d.width = Math.max(8, Math.round((d.hi - d.lo) / span * 100))
      d.hiT = W.fmtT(d.hi, u); d.loT = W.fmtT(d.lo, u)
    })
    const lv = W.aqiLevel(c.aqi)
    const mine = W.myCities()
    this.setData({
      city: c, unit: u,
      bg: W.COND[c.cond].bg,
      condName: W.COND[c.cond].name,
      temp: W.fmtT(t, u),
      feels: W.fmtT(t + (c.hum > 70 ? 2 : c.lv >= 4 ? -3 : -1), u),
      hiT: W.fmtT(c.hi, u), loT: W.fmtT(c.lo, u),
      hours, days,
      aqi: { v: c.aqi, name: lv.name, color: lv.color, tip: lv.tip, pct: Math.min(100, Math.round(c.aqi / 300 * 100)) },
      life: W.lifeIndex(c),
      mine, ci: mine.indexOf(id),
      updated: (now.getHours() < 10 ? '0' : '') + now.getHours() + ':' + (now.getMinutes() < 10 ? '0' : '') + now.getMinutes() + ' 更新'
    })
    wx.setNavigationBarColor && wx.setNavigationBarColor({ frontColor: '#ffffff', backgroundColor: '#2f7ff0' })
  },
  onCity() { wx.navigateTo({ url: '/pages/city/city' }) },
  onAir() { wx.navigateTo({ url: '/pages/air/air?id=' + this.data.city.id }) },
  onLife(e) { wx.navigateTo({ url: '/pages/life/life?id=' + this.data.city.id + '&key=' + e.currentTarget.dataset.key }) },
  onSettings() { wx.navigateTo({ url: '/pages/settings/settings' }) },
  onDot(e) {
    const id = this.data.mine[e.currentTarget.dataset.i]
    W.setCurrent(id); this.render()
  },
  onHour(e) {
    const h = this.data.hours[e.currentTarget.dataset.i]
    wx.showToast({ title: h.label + ' ' + W.COND[h.cond === 'night' ? 'sunny' : h.cond].name + ' ' + h.tt + '°' + (h.pop ? ' 降水' + h.pop + '%' : ''), icon: 'none' })
  }
})
