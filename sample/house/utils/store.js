// 本地状态：收藏 / 足迹 / 看房预约 / 联系人 / 购房需求 / 房源订阅 / 卖房委托，全部存在 storage 里
const D = require('./data.js')
const K = {
  favs: 'house_favs', history: 'house_history', bookings: 'house_bookings', contact: 'house_contact',
  profile: 'house_profile', subs: 'house_subs', entrusts: 'house_entrusts', seeded: 'house_seeded'
}
const STATUS_TEXT = { pending: '待带看', done: '已带看', cancelled: '已取消' }
const ENTRUST_TEXT = { reviewing: '核验中', listed: '已上架', withdrawn: '已撤销' }
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']
function pad(n) {
  return n < 10 ? '0' + n : '' + n
}
function dateStr(ts) {
  const d = new Date(ts)
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate())
}
function dateTime(ts) {
  const d = new Date(ts)
  return dateStr(ts) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes())
}
// getStorageSync 缺省给 ''：数组 / 对象各自兜底，不用 `||`（空数组、0 都是合法值）
function list(k) {
  const v = wx.getStorageSync(k)
  return Array.isArray(v) ? v : []
}
function obj(k) {
  const v = wx.getStorageSync(k)
  return v && typeof v === 'object' && !Array.isArray(v) ? v : {}
}
// 首次打开放两条演示数据：一条已带看的预约、两个房源订阅（「我的」里一进来不是空的）
function ensureSeed() {
  if (wx.getStorageSync(K.seeded)) return
  wx.setStorageSync(K.seeded, true)
  if (!list(K.bookings).length) {
    const day = Date.now() - 6 * 86400000
    wx.setStorageSync(K.bookings, [{
      id: 1, houseId: 'h2', community: '西溪诚园', cover: '/images/h2_living.png', layout: '2室1厅1卫', area: 89,
      day: dateStr(day), time: WEEK[new Date(day).getDay()] + ' ' + (new Date(day).getMonth() + 1) + '/' + new Date(day).getDate() + ' 14:00-16:00', slot: 2,
      name: '周一凡', phone: '13757112345', gender: 0, people: '2 人', pickup: false, note: '',
      agent: '陆明轩', agentPhone: '139 5810 7712', status: 'done', createdAt: day - 2 * 86400000
    }])
  }
  if (!list(K.subs).length) {
    wx.setStorageSync(K.subs, [
      { id: 1, type: 'second', district: '滨江', lIndex: 3, pIndex: 0, createdAt: Date.now() - 20 * 86400000 },
      { id: 2, type: 'rent', district: '不限', lIndex: 2, pIndex: 1, createdAt: Date.now() - 9 * 86400000 }
    ])
  }
}
// ───────── 收藏 / 足迹 ─────────
function getFavs() {
  return list(K.favs)
}
function isFav(id) {
  return getFavs().indexOf(id) >= 0
}
// 返回切换后是否收藏
function toggleFav(id) {
  const favs = getFavs()
  const i = favs.indexOf(id)
  if (i >= 0) favs.splice(i, 1)
  else favs.unshift(id)
  wx.setStorageSync(K.favs, favs)
  return i < 0
}
function getHistory() {
  return list(K.history)
}
function pushHistory(id) {
  const h = getHistory().filter(x => x !== id)
  h.unshift(id)
  wx.setStorageSync(K.history, h.slice(0, 20))
}
function removeHistory(id) {
  wx.setStorageSync(K.history, getHistory().filter(x => x !== id))
}
function clearHistory() {
  wx.setStorageSync(K.history, [])
}
// ───────── 看房预约 ─────────
// 看房日已过的待带看自动转成已带看（经纪人带看完成）
function getBookings() {
  ensureSeed()
  const today = dateStr(Date.now())
  let changed = false
  const l = list(K.bookings).map(b => {
    if (b.status === 'pending' && b.day < today) {
      changed = true
      return Object.assign({}, b, { status: 'done' })
    }
    return b
  })
  if (changed) wx.setStorageSync(K.bookings, l)
  return l.map(b => Object.assign({}, b, { statusText: STATUS_TEXT[b.status] }))
}
function getBooking(id) {
  return getBookings().find(b => String(b.id) === String(id)) || null
}
function saveBookings(l) {
  wx.setStorageSync(K.bookings, l.map(b => {
    const x = Object.assign({}, b)
    delete x.statusText
    return x
  }))
}
function addBooking(b) {
  const l = getBookings()
  const id = l.reduce((m, x) => Math.max(m, Number(x.id) || 0), 0) + 1
  const rec = Object.assign({ id, status: 'pending', createdAt: Date.now() }, b)
  l.unshift(rec)
  saveBookings(l)
  return rec
}
function updateBooking(id, patch) {
  const l = getBookings()
  const b = l.find(x => String(x.id) === String(id))
  if (!b) return null
  Object.assign(b, patch)
  saveBookings(l)
  return b
}
function cancelBooking(id) {
  return updateBooking(id, { status: 'cancelled', cancelledAt: Date.now() })
}
function pendingFor(houseId) {
  return getBookings().find(b => b.houseId === houseId && b.status === 'pending') || null
}
function getContact() {
  return obj(K.contact)
}
function saveContact(c) {
  wx.setStorageSync(K.contact, { name: c.name, phone: c.phone, gender: c.gender })
}
// ───────── 购房需求 ─────────
const NEEDS = ['刚需首套', '改善置换', '投资', '租房']
function getProfile() {
  const p = obj(K.profile)
  return { name: p.name || '周一凡', need: p.need || NEEDS[0] }
}
function setNeed(need) {
  const p = obj(K.profile)
  p.need = need
  wx.setStorageSync(K.profile, p)
}
// ───────── 房源订阅 ─────────
function getSubs() {
  ensureSeed()
  return list(K.subs).map(s => Object.assign({}, s, {
    text: D.condText(s), count: D.filter(s).length, date: dateStr(s.createdAt)
  }))
}
// 同样的条件只订一次：返回 null 表示已经订过
function addSub(c) {
  const l = list(K.subs)
  const same = l.find(s => s.type === c.type && s.district === c.district && s.lIndex === c.lIndex && s.pIndex === c.pIndex)
  if (same) return null
  const id = l.reduce((m, x) => Math.max(m, Number(x.id) || 0), 0) + 1
  const s = { id, type: c.type, district: c.district || '不限', lIndex: c.lIndex || 0, pIndex: c.pIndex || 0, createdAt: Date.now() }
  l.unshift(s)
  wx.setStorageSync(K.subs, l)
  return s
}
function removeSub(id) {
  wx.setStorageSync(K.subs, list(K.subs).filter(s => String(s.id) !== String(id)))
}
// ───────── 卖房 / 出租委托 ─────────
function getEntrusts() {
  return list(K.entrusts).map(e => Object.assign({}, e, { statusText: ENTRUST_TEXT[e.status], date: dateTime(e.createdAt) }))
}
function addEntrust(e) {
  const l = list(K.entrusts)
  const id = l.reduce((m, x) => Math.max(m, Number(x.id) || 0), 0) + 1
  const rec = Object.assign({ id, status: 'reviewing', createdAt: Date.now() }, e)
  l.unshift(rec)
  wx.setStorageSync(K.entrusts, l)
  return rec
}
function withdrawEntrust(id) {
  const l = list(K.entrusts)
  const e = l.find(x => String(x.id) === String(id))
  if (e) e.status = 'withdrawn'
  wx.setStorageSync(K.entrusts, l)
}
module.exports = {
  K, NEEDS, dateStr, dateTime,
  getFavs, isFav, toggleFav, getHistory, pushHistory, removeHistory, clearHistory,
  getBookings, getBooking, addBooking, updateBooking, cancelBooking, pendingFor, getContact, saveContact,
  getProfile, setNeed, getSubs, addSub, removeSub, getEntrusts, addEntrust, withdrawEntrust
}
