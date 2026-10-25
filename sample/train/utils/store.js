// 乘车人、订单、查询条件：落在本地 storage（tr_passengers / tr_orders / tr_query）
const D = require('./data.js')
const SEED_PASSENGERS = [
  { id: 1, name: '李明', idcard: '330106199003071236', phone: '13812345678', self: true }
]
function passengers() {
  let list = wx.getStorageSync('tr_passengers')
  if (!list || !list.length) {
    list = SEED_PASSENGERS.slice()
    wx.setStorageSync('tr_passengers', list)
  }
  return list
}
function addPassenger(p) {
  const list = passengers()
  const id = list.reduce((m, x) => Math.max(m, x.id), 0) + 1
  const item = Object.assign({ id, self: false }, p)
  list.push(item)
  wx.setStorageSync('tr_passengers', list)
  return item
}
function removePassenger(id) {
  wx.setStorageSync('tr_passengers', passengers().filter(p => p.id !== id))
}
// 身份证中间打码：3301**********1236
function maskId(s) {
  return s.slice(0, 4) + '**********' + s.slice(-4)
}
function orders() {
  return wx.getStorageSync('tr_orders') || []
}
function saveOrders(list) {
  wx.setStorageSync('tr_orders', list)
}
function addOrder(o) {
  const list = orders()
  const order = Object.assign({ id: 'E' + Date.now() + Math.floor(Math.random() * 90 + 10), status: 'paid', created: Date.now() }, o)
  list.unshift(order)
  saveOrders(list)
  return order
}
function cancelOrder(id) {
  saveOrders(orders().map(o => (o.id === id ? Object.assign({}, o, { status: 'cancelled' }) : o)))
}
// 状态：已取消 / 发车日已过 → 已完成；其余待出行
function orderState(o) {
  if (o.status === 'cancelled') return { done: true, text: '已取消' }
  if (o.date < D.today()) return { done: true, text: '已出行' }
  return { done: false, text: '待出行' }
}
const DEFAULT_QUERY = { from: '北京', to: '上海', highSpeed: false }
function query() {
  return Object.assign({}, DEFAULT_QUERY, wx.getStorageSync('tr_query') || {})
}
function saveQuery(q) {
  wx.setStorageSync('tr_query', q)
}
module.exports = { passengers, addPassenger, removePassenger, maskId, orders, addOrder, cancelOrder, orderState, query, saveQuery }
