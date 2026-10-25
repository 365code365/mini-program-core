// 计费：15 分钟内免费；之后首小时 first 元，超出部分每半小时 half 元（不足半小时按半小时），每 24 小时封顶 cap
const { FREE_MIN } = require('./data.js')

function dayFee(min, lot) {
  if (min <= 0) return 0
  if (min <= 60) return lot.first
  return Math.min(lot.first + Math.ceil((min - 60) / 30) * lot.half, lot.cap)
}
function calcFee(min, lot) {
  if (min <= FREE_MIN) return 0
  const days = Math.floor(min / 1440)
  return days * lot.cap + dayFee(min % 1440, lot)
}
function toMin(hhmm) {
  const p = String(hhmm).split(':')
  return Number(p[0]) * 60 + Number(p[1])
}
// 入场到现在的分钟数（跨零点按次日算）
function between(inTime, now) {
  let d = toMin(now) - toMin(inTime)
  if (d < 0) d += 1440
  return d
}
function durText(min) {
  const h = Math.floor(min / 60)
  const m = min % 60
  return (h ? h + '小时' : '') + m + '分钟'
}
function money(n) {
  return '¥' + Number(n).toFixed(2)
}
// 收费规则的文字说明
function ruleText(lot) {
  return ['15 分钟内免费', '首小时 ' + lot.first + ' 元', '之后每半小时 ' + lot.half + ' 元', '24 小时封顶 ' + lot.cap + ' 元']
}
// 券抵扣：返回 { off, ok, why }
function couponOff(c, fee, lot) {
  if (!c || (!c.off && !c.firstHour)) return { off: 0, ok: true, why: '' }
  if (fee <= 0) return { off: 0, ok: false, why: '本次免费，无需用券' }
  if (fee < c.min) return { off: 0, ok: false, why: '未满 ' + c.min + ' 元，不能使用' }
  const off = c.firstHour ? Math.min(lot.first, fee) : Math.min(c.off, fee)
  return { off, ok: true, why: '' }
}

module.exports = { calcFee, toMin, between, durText, money, ruleText, couponOff }
