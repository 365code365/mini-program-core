// 迷你播放条：发现 / 曲库 / 歌单页都 include 了 templates/mini-bar.wxml。
// withMini 给页面混入订阅播放器与模板的事件处理（templates/mini-bar.js：点条进播放页、条上播放/暂停），页面自己的 onShow / onHide 照常执行
const P = require('./player.js')
const bar = require('../templates/mini-bar.js')
function withMini(page) {
  const own = { onShow: page.onShow, onHide: page.onHide, onUnload: page.onUnload }
  page.data = Object.assign({ mini: null, miniPlaying: false, miniPct: 0 }, page.data || {})
  page.onShow = function () {
    if (!this.__mini) {
      this.__mini = snap => {
        const s = snap.song
        this.setData({ mini: s, miniPlaying: snap.playing, miniPct: s ? Math.floor(snap.pos * 100 / s.duration) : 0 })
      }
    }
    P.on(this.__mini)
    if (own.onShow) own.onShow.apply(this, arguments)
  }
  page.onHide = function () {
    if (this.__mini) P.off(this.__mini)
    if (own.onHide) own.onHide.apply(this, arguments)
  }
  page.onUnload = function () {
    if (this.__mini) P.off(this.__mini)
    if (own.onUnload) own.onUnload.apply(this, arguments)
  }
  // 模板里绑定的 onMiniTap / onMiniToggle 写在 templates/mini-bar.js，混进页面
  Object.keys(bar).forEach(k => { page[k] = bar[k] })
  return page
}
module.exports = { withMini }
