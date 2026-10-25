// 表单校验与日期计算
const W = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2]
const CODES = '10X98765432'

function pad(n) {
  return n < 10 ? '0' + n : '' + n
}

// 18 位居民身份证：格式 + 出生日期合法 + 末位校验码（GB 11643）
function idCard(s) {
  s = String(s || '').toUpperCase()
  if (!/^\d{17}[\dX]$/.test(s)) return false
  const y = Number(s.slice(6, 10))
  const m = Number(s.slice(10, 12))
  const d = Number(s.slice(12, 14))
  const dt = new Date(y, m - 1, d)
  if (dt.getFullYear() !== y || dt.getMonth() !== m - 1 || dt.getDate() !== d) return false
  let sum = 0
  for (let i = 0; i < 17; i++) sum += Number(s[i]) * W[i]
  return CODES[sum % 11] === s[17]
}
// 从身份证号取出生日期与性别（第 17 位奇数为男）
function idInfo(s) {
  s = String(s).toUpperCase()
  return { birth: s.slice(6, 10) + '-' + s.slice(10, 12) + '-' + s.slice(12, 14), gender: Number(s[16]) % 2 ? '男' : '女' }
}
function otherId(s) {
  return /^[A-Za-z0-9]{6,20}$/.test(String(s || ''))
}
function phone(s) {
  return /^1[3-9]\d{9}$/.test(String(s || ''))
}
function name(s) {
  s = String(s || '').trim()
  return s.length >= 2 && s.length <= 20
}
// 到 on 那天为止的周岁
function age(birth, on) {
  const b = birth.split('-').map(Number)
  const o = on.split('-').map(Number)
  let a = o[0] - b[0]
  if (o[1] < b[1] || (o[1] === b[1] && o[2] < b[2])) a--
  return a
}
// 「2027-03-15T08:00:00」→ 本地时间的 Date（不依赖 Date.parse 对无时区字符串的解释）
function parseLocal(s) {
  const m = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2}))?)?$/.exec(String(s || ''))
  if (!m) return null
  return new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]), Number(m[4] || 0), Number(m[5] || 0), Number(m[6] || 0))
}
// 毫秒差 → 天时分秒（两位补零的字符串另给一份用于显示）
function split(ms) {
  const t = Math.max(0, Math.floor(ms / 1000))
  const d = Math.floor(t / 86400)
  const h = Math.floor((t % 86400) / 3600)
  const m = Math.floor((t % 3600) / 60)
  const s = t % 60
  return { d, h, m, s, text: d + '天' + pad(h) + '时' + pad(m) + '分' + pad(s) + '秒' }
}
// 「03:30」→「3 小时 30 分」
function bestText(hhmm) {
  if (!hhmm) return '未填写'
  const p = hhmm.split(':').map(Number)
  return (p[0] ? p[0] + ' 小时 ' : '') + p[1] + ' 分'
}
function maskId(s) {
  s = String(s)
  return s.length > 8 ? s.slice(0, 4) + '**********'.slice(0, s.length - 8) + s.slice(-4) : s
}

module.exports = { pad, idCard, idInfo, otherId, phone, name, age, parseLocal, split, bestText, maskId }
