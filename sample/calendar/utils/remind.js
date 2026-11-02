// 提醒与搜索要用的推算：提醒时刻、下一次发生日期。都只依赖 store 里的重复规则
const D = require('./date.js')
const S = require('./store.js')
// 与 S.REMINDS 一一对应：提前多少分钟（null = 不提醒）
const OFFSETS = [null, 0, 5, 15, 60, 1440]
function toMin(hhmm) {
  const p = String(hhmm || '09:00').split(':')
  return Number(p[0]) * 60 + Number(p[1])
}
function hhmm(min) {
  return D.pad(Math.floor(min / 60)) + ':' + D.pad(min % 60)
}
// 这一次发生的提醒时刻：{ date, time, abs }，abs 是相对 1970 的分钟数（只用来排序、比先后）
function remindAt(ev, date) {
  const off = OFFSETS[ev.remind]
  if (off === null || off === undefined) return null
  // 全天日程按当天 09:00 算
  let min = (ev.allDay ? 9 * 60 : toMin(ev.start)) - off
  let d = date
  while (min < 0) {
    min += 1440
    d = D.addDays(d, -1)
  }
  return { date: d, time: hhmm(min), abs: D.parse(d).getTime() / 60000 + min }
}
// 「现在」：日期跟着 store.today()（可注入），时刻用系统时钟
function nowAbs() {
  const n = new Date()
  return D.parse(S.today()).getTime() / 60000 + n.getHours() * 60 + n.getMinutes()
}
// 从 from 起（含）最近一次发生的日期；往后找一年，找不到返回 ''
function nextOn(ev, from) {
  for (let i = 0; i <= 366; i++) {
    const d = D.addDays(from, i)
    if (S.occursOn(ev, d)) return d
  }
  return ''
}
module.exports = { OFFSETS, remindAt, nowAbs, nextOn, toMin, hhmm }
