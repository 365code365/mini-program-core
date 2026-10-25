// 收藏与最近播放：落在本地 storage（mu_favs / mu_recent，都是歌曲 id 数组，新的在前）
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
module.exports = { favs, isFav, toggleFav, recent, addRecent, clearRecent }
