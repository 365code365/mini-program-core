// templates/mini-bar.wxml 里绑定的处理函数，和模板放在一起：<include> 进来的模板没有自己的 JS，
// 事件交给 include 它的页面处理，所以由 utils/mini.js 的 withMini 把这里的函数混进页面（发现 / 曲库 / 歌单页）。
// 新加一个 include 了 mini-bar 的页面，必须用 withMini 包一下，否则点条会报「找不到 onMiniTap」。
const P = require('../utils/player.js')
const handlers = {
  // 点条身：进播放页
  onMiniTap() {
    wx.navigateTo({ url: '/pages/player/player' })
  },
  // catchtap：只切播放状态，不冒泡到整条的「进播放页」
  onMiniToggle() {
    P.toggle()
  }
}
module.exports = handlers
