// 收藏与最近播放：落在本地 storage（mu_favs / mu_recent，都是歌曲 id 数组，新的在前；mu_pl_favs 是收藏的歌单 id）
const RECENT_MAX = 20
function favs() {
  return wx.getStorageSync('mu_favs') || []
}
function isFav(id) {
  return favs().indexOf(id) >= 0
}
// 切换收藏，返回切换后的状态
function toggleFav(id) {
  const list = favs()
  const i = list.indexOf(id)
  if (i >= 0) list.splice(i, 1)
  else list.unshift(id)
  wx.setStorageSync('mu_favs', list)
  return i < 0
}
function recent() {
  return wx.getStorageSync('mu_recent') || []
}
function addRecent(id) {
  const list = recent().filter(x => x !== id)
  list.unshift(id)
  wx.setStorageSync('mu_recent', list.slice(0, RECENT_MAX))
}
function clearRecent() {
  wx.removeStorageSync('mu_recent')
}
function plFavs() {
  const v = wx.getStorageSync('mu_pl_favs')
  return Array.isArray(v) ? v : []
}
function isPlFav(id) {
  return plFavs().indexOf(id) >= 0
}
function togglePlFav(id) {
  const list = plFavs()
  const on = list.indexOf(id) < 0
  wx.setStorageSync('mu_pl_favs', on ? [id].concat(list) : list.filter(x => x !== id))
  return on
}
module.exports = { favs, isFav, toggleFav, recent, addRecent, clearRecent, plFavs, isPlFav, togglePlFav }
