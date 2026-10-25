const M = require('./utils/market.js')
App({
  globalData: {},
  onLaunch() {
    // 每秒跳一次行情，并通知最上面那一页（页面实现了 onTick 就刷新）
    this._timer = setInterval(() => {
      M.tick()
      const pages = getCurrentPages()
      const top = pages[pages.length - 1]
      if (top && typeof top.onTick === 'function') top.onTick()
    }, 1000)
  }
})
