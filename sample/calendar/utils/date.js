// 日期工具：全部按「YYYY-MM-DD」字符串在页面之间传递，避免时区把日期算偏一天
const WEEK = ['日', '一', '二', '三', '四', '五', '六']

function pad(n) {
  return n < 10 ? '0' + n : '' + n
}
function fmt(y, m, d) {
  return y + '-' + pad(m) + '-' + pad(d)
}
function fmtDate(dt) {
  return fmt(dt.getFullYear(), dt.getMonth() + 1, dt.getDate())
}
// 只用年月日构造本地日期（new Date('2026-03-15') 会按 UTC 解析）
function parse(s) {
  const p = String(s).split('-')
  return new Date(Number(p[0]), Number(p[1]) - 1, Number(p[2]))
}
function daysInMonth(y, m) {
  return new Date(y, m, 0).getDate()
}
function addDays(s, n) {
  const d = parse(s)
  d.setDate(d.getDate() + n)
  return fmtDate(d)
}
function weekday(s) {
  return parse(s).getDay()
}
// 年月加减：返回 { year, month }
function addMonths(year, month, n) {
  const t = year * 12 + (month - 1) + n
  return { year: Math.floor(t / 12), month: (t % 12) + 1 }
}
// 「03-16 周一」
function label(s) {
  return s.slice(5) + ' 周' + WEEK[weekday(s)]
}
// 「3月16日 周一」
function longLabel(s) {
  const p = s.split('-')
  return Number(p[1]) + '月' + Number(p[2]) + '日 周' + WEEK[weekday(s)]
}
// 两个日期相差的天数（b - a）
function diffDays(a, b) {
  return Math.round((parse(b) - parse(a)) / 86400000)
}

/**
 * 月视图的 6×7 格子。weekStart：1 = 周一开头，0 = 周日开头。
 * 第一格是「本月 1 号所在那一周的周起始日」，不足的用上月补位，末尾用下月补满 42 格。
 */
function monthGrid(year, month, weekStart) {
  const first = fmt(year, month, 1)
  const offset = (weekday(first) - weekStart + 7) % 7
  const start = addDays(first, -offset)
  const cells = []
  for (let i = 0; i < 42; i++) {
    const date = addDays(start, i)
    cells.push({ date, day: Number(date.slice(8)), cur: Number(date.slice(5, 7)) === month })
  }
  return cells
}
function weekHeads(weekStart) {
  const out = []
  for (let i = 0; i < 7; i++) out.push(WEEK[(weekStart + i) % 7])
  return out
}

module.exports = { WEEK, pad, fmt, fmtDate, parse, daysInMonth, addDays, weekday, addMonths, label, longLabel, diffDays, monthGrid, weekHeads }
