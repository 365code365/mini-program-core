// 播放器：全局唯一的播放状态（模块级单例），用 setInterval 每秒推进 1 秒模拟播放。
// 页面通过 on(fn) 订阅状态，onHide / onUnload 时 off(fn) 退订，避免给已销毁的页面 setData。
const D = require('./songs.js')
const S = require('./store.js')
const MODES = [
  { key: 'seq', text: '顺序播放', icon: '顺' },
  { key: 'one', text: '单曲循环', icon: '单' },
  { key: 'rand', text: '随机播放', icon: '随' }
]
const st = { queue: [], index: -1, playing: false, pos: 0, mode: 0 }
const subs = []
let timer = null

function current() {
  return st.index >= 0 ? D.song(st.queue[st.index]) : null
}
function snapshot() {
  const s = current()
  return {
    song: s,
    playing: st.playing,
    pos: st.pos,
    posText: D.fmt(st.pos),
    mode: st.mode,
    modeText: MODES[st.mode].text,
    modeIcon: MODES[st.mode].icon,
    index: st.index,
    queue: st.queue.slice()
  }
}
function emit() {
  const snap = snapshot()
  subs.slice().forEach(fn => fn(snap))
}
function on(fn) {
  if (subs.indexOf(fn) < 0) subs.push(fn)
  fn(snapshot())
}
function off(fn) {
  const i = subs.indexOf(fn)
  if (i >= 0) subs.splice(i, 1)
}
function ensureTimer() {
  if (!timer) timer = setInterval(tick, 1000)
}
function tick() {
  const s = current()
  if (!st.playing || !s) return
  st.pos += 1
  if (st.pos >= s.duration) {
    // 播完：单曲循环从头再来，其余模式自动下一首
    if (st.mode === 1) st.pos = 0
    else return next()
  }
  emit()
}
function start(i) {
  st.index = i
  st.pos = 0
  st.playing = true
  S.addRecent(st.queue[i])
  ensureTimer()
  emit()
}
// 用一组歌曲 id 作为播放队列，从第 i 首开始放
function playList(ids, i) {
  if (!ids || !ids.length) return
  const cur = current()
  // 点的正是正在放的那首：不打断，只保证在播
  if (cur && cur.id === ids[i] && st.queue.join() === ids.join()) {
    if (!st.playing) toggle()
    return
  }
  st.queue = ids.slice()
  start(Math.max(0, Math.min(i, ids.length - 1)))
}
function randomIndex() {
  const n = st.queue.length
  if (n < 2) return 0
  let i = st.index
  while (i === st.index) i = Math.floor(Math.random() * n)
  return i
}
// 手动「下一首」：随机模式随机挑一首别的，其余（含单曲循环）按顺序
function next() {
  const n = st.queue.length
  if (!n) return
  start(st.mode === 2 ? randomIndex() : (st.index + 1) % n)
}
function prev() {
  const n = st.queue.length
  if (!n) return
  start(st.mode === 2 ? randomIndex() : (st.index - 1 + n) % n)
}
function toggle() {
  if (!current()) return
  st.playing = !st.playing
  ensureTimer()
  emit()
}
function seek(sec) {
  const s = current()
  if (!s) return
  st.pos = Math.max(0, Math.min(Math.floor(sec), s.duration - 1))
  emit()
}
function cycleMode() {
  st.mode = (st.mode + 1) % MODES.length
  emit()
  return MODES[st.mode]
}
module.exports = { MODES, on, off, playList, next, prev, toggle, seek, cycleMode, snapshot }
