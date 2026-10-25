// 本地 storage：我的车辆 pk_cars、在场记录 pk_sessions、订单 pk_orders、各停车场空位 pk_free
const { LOTS, lotById } = require('./data.js')

function get(key, seed) {
  const v = wx.getStorageSync(key)
  if (v === '' || v === undefined || v === null) {
    const s = typeof seed === 'function' ? seed() : seed
    wx.setStorageSync(key, s)
    return s
  }
  return v
}
function set(key, v) {
  wx.setStorageSync(key, v)
}

function cars() {
  return get('pk_cars', [
    { plate: '京A12345', newEnergy: false },
    { plate: '沪AD12345', newEnergy: true }
  ])
}
function saveCars(list) { set('pk_cars', list) }

// 在场记录：车辆已经进了哪个停车场、几点入场（模拟道闸推送）
function sessions() {
  return get('pk_sessions', [
    { plate: '京A12345', lotId: 1, inTime: '15:10' },
    { plate: '沪AD12345', lotId: 3, inTime: '09:40' }
  ])
}
function saveSessions(list) { set('pk_sessions', list) }
function sessionOf(plate) {
  return sessions().find(s => s.plate === plate) || null
}

function orders() {
  return get('pk_orders', [
    { id: 'P1000', type: 'pay', status: 'done', statusText: '已支付', plate: '京A12345', lotId: 3, lotName: '市民中心 P3 停车楼', desc: '08:20 入场 · 停车 2小时45分钟', amount: 14 }
  ])
}
function saveOrders(list) { set('pk_orders', list) }
// 订单号：类型前缀 + 递增序号（演示环境可确定）
function nextOrderId(prefix) {
  return prefix + (1001 + orders().length)
}

function freeMap() {
  const m = {}
  LOTS.forEach(l => { m[l.id] = l.free })
  return get('pk_free', m)
}
function freeOf(id) {
  return Number(freeMap()[id])
}
function addFree(id, n) {
  const m = freeMap()
  const lot = lotById(id)
  m[id] = Math.max(0, Math.min(lot.total, Number(m[id]) + n))
  set('pk_free', m)
}

module.exports = { cars, saveCars, sessions, saveSessions, sessionOf, orders, saveOrders, nextOrderId, freeMap, freeOf, addFree }
