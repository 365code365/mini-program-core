// 收藏、搜索历史、做过的菜：落在本地 storage
// rc_favs：[{ id, pinned, time }]；rc_history：关键词数组（新的在前、去重、最多 10 条）；rc_cooked：{ id: 次数 }
const HISTORY_MAX = 10
function favs() {
  return wx.getStorageSync('rc_favs') || []
}
function isFav(id) {
  return favs().some(f => f.id === id)
}
function toggleFav(id) {
  const list = favs()
  const i = list.findIndex(f => f.id === id)
  if (i >= 0) list.splice(i, 1)
  else list.unshift({ id, pinned: false, time: Date.now() })
  wx.setStorageSync('rc_favs', list)
  return i < 0
}
function removeFav(id) {
  wx.setStorageSync('rc_favs', favs().filter(f => f.id !== id))
}
// 置顶 / 取消置顶：置顶的排最前（多个置顶按置顶时间新的在前）
function togglePin(id) {
  const list = favs().map(f => (f.id === id ? Object.assign({}, f, { pinned: !f.pinned, pinTime: Date.now() }) : f))
  wx.setStorageSync('rc_favs', list)
  return list.find(f => f.id === id).pinned
}
function sortedFavs() {
  return favs().slice().sort((a, b) => (b.pinned - a.pinned) || (b.pinned ? (b.pinTime || 0) - (a.pinTime || 0) : b.time - a.time))
}
function history() {
  return wx.getStorageSync('rc_history') || []
}
function addHistory(kw) {
  const list = [kw].concat(history().filter(x => x !== kw)).slice(0, HISTORY_MAX)
  wx.setStorageSync('rc_history', list)
  return list
}
function clearHistory() {
  wx.removeStorageSync('rc_history')
}
function cooked() {
  return wx.getStorageSync('rc_cooked') || {}
}
function addCooked(id) {
  const m = cooked()
  m[id] = (m[id] || 0) + 1
  wx.setStorageSync('rc_cooked', m)
  return m[id]
}
module.exports = { favs, isFav, toggleFav, removeFav, togglePin, sortedFavs, history, addHistory, clearHistory, cooked, addCooked }
