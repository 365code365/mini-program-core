// 本地数据：书架、阅读进度、书签、阅读设置、阅读时长
const B = require('./books.js')

const THEMES = [
  { key: 'day', name: '日间', bg: '#f7f3ea', fg: '#2e2a24', bar: '#ffffff' },
  { key: 'night', name: '夜间', bg: '#1b1d22', fg: '#8e939c', bar: '#24272d' },
  { key: 'eye', name: '护眼', bg: '#cfe8cc', fg: '#2c3b2a', bar: '#e1f0de' }
]
const FONT_MIN = 14
const FONT_MAX = 26

function pad(n) {
  return n < 10 ? '0' + n : '' + n
}

function today() {
  const d = new Date()
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate())
}

// 书架：首次打开放 4 本，并给其中 3 本一点进度
function loadShelf() {
  let s = wx.getStorageSync('rd_shelf')
  if (!s) {
    s = ['b1', 'b2', 'b5', 'b4']
    wx.setStorageSync('rd_shelf', s)
    wx.setStorageSync('rd_progress', {
      b2: { chapter: 6, scrollTop: 0, pct: 20 },
      b5: { chapter: 20, scrollTop: 0, pct: 67 },
      b4: { chapter: 1, scrollTop: 0, pct: 3 }
    })
  }
  return s
}

function inShelf(id) {
  return loadShelf().indexOf(id) >= 0
}

function addShelf(id) {
  const s = loadShelf()
  if (s.indexOf(id) < 0) {
    s.unshift(id)
    wx.setStorageSync('rd_shelf', s)
  }
}

function removeShelf(id) {
  wx.setStorageSync('rd_shelf', loadShelf().filter(x => x !== id))
}

function loadProgress() {
  loadShelf()
  return wx.getStorageSync('rd_progress') || {}
}

function progressOf(id) {
  return loadProgress()[id] || null
}

// 进度百分比 = (已读完的章 + 本章滚动比例) / 总章数
function saveProgress(id, chapter, scrollTop, ratio) {
  const book = B.get(id)
  const all = loadProgress()
  const pct = Math.min(100, Math.round((chapter + Math.max(0, Math.min(1, ratio || 0))) / book.chapters * 100))
  all[id] = { chapter, scrollTop: Math.round(scrollTop), pct }
  wx.setStorageSync('rd_progress', all)
  return all[id]
}

function loadSettings() {
  return Object.assign({ fontSize: 18, theme: 'day' }, wx.getStorageSync('rd_settings') || {})
}

function saveSettings(s) {
  wx.setStorageSync('rd_settings', s)
}

function theme(key) {
  return THEMES.find(t => t.key === key) || THEMES[0]
}

function loadBookmarks() {
  return wx.getStorageSync('rd_bookmarks') || []
}

// 同一本书同一章只留一个书签；返回 false 表示已经有了
function addBookmark(bm) {
  const list = loadBookmarks()
  if (list.some(x => x.bookId === bm.bookId && x.chapter === bm.chapter)) return false
  list.unshift(Object.assign({ id: 'm' + Date.now() + list.length, at: today() }, bm))
  wx.setStorageSync('rd_bookmarks', list)
  return true
}

function removeBookmark(id) {
  wx.setStorageSync('rd_bookmarks', loadBookmarks().filter(x => x.id !== id))
}

// 阅读时长（秒）：总计 + 按天
function addReading(seconds) {
  if (seconds <= 0) return
  const s = wx.getStorageSync('rd_stats') || { total: 0, days: {} }
  s.total += seconds
  const d = today()
  s.days[d] = (s.days[d] || 0) + seconds
  wx.setStorageSync('rd_stats', s)
}

function loadStats() {
  const s = wx.getStorageSync('rd_stats') || { total: 0, days: {} }
  return { total: s.total, today: s.days[today()] || 0, days: Object.keys(s.days).length }
}

module.exports = {
  THEMES, FONT_MIN, FONT_MAX, loadShelf, inShelf, addShelf, removeShelf, loadProgress, progressOf, saveProgress,
  loadSettings, saveSettings, theme, loadBookmarks, addBookmark, removeBookmark, addReading, loadStats
}
