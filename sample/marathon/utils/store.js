// 报名记录落在本地 storage（mr_entries）；各组别已报人数 = 基数 + 本机有效报名
const { GROUPS } = require('./data.js')

function entries() {
  const v = wx.getStorageSync('mr_entries')
  return Array.isArray(v) ? v : []
}
function saveEntries(list) {
  wx.setStorageSync('mr_entries', list)
}
function active(e) {
  return e.status !== 'canceled'
}
function takenOf(gi) {
  return GROUPS[gi].taken + entries().filter(e => active(e) && e.group === gi).length
}
// 报名号：组别前缀 + 递增序号（演示环境可确定），如 H10001
function nextNo(gi) {
  return GROUPS[gi].code + (10001 + entries().length)
}
function findByNo(no) {
  return entries().find(e => e.no === no) || null
}
function update(no, patch) {
  const list = entries()
  const e = list.find(i => i.no === no)
  if (!e) return null
  Object.assign(e, patch)
  saveEntries(list)
  return e
}
// 同一证件号已有有效报名
function duplicated(idNo) {
  return entries().some(e => active(e) && e.idNo.toUpperCase() === String(idNo).toUpperCase())
}

const STATUS = { unpaid: '待缴费', paid: '报名成功', canceled: '已取消' }

module.exports = { entries, saveEntries, takenOf, nextNo, findByNo, update, duplicated, STATUS }
