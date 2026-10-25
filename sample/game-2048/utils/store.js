// 本地存储：设置、最高分、历史成绩
const SIZES = [3, 4, 5]
const THEMES = [
  { key: 'classic', name: '经典暖棕', board: '#bbada0', cell: '#cdc1b4', accent: '#8f7a66' },
  { key: 'ocean', name: '深海蓝', board: '#6c8ead', cell: '#9db4c9', accent: '#3d6185' },
  { key: 'forest', name: '森林绿', board: '#8aa37b', cell: '#b4c7a8', accent: '#4f6f3f' }
]
const DEFAULT_SETTINGS = { size: 4, anim: true, theme: 'classic' }

function loadSettings() {
  const s = wx.getStorageSync('g2048_settings')
  return Object.assign({}, DEFAULT_SETTINGS, s || {})
}

function saveSettings(s) {
  wx.setStorageSync('g2048_settings', s)
}

function theme(key) {
  return THEMES.find(t => t.key === key) || THEMES[0]
}

// 最高分按棋盘尺寸分开记（3×3 与 5×5 不可比）
function bestKey(size) {
  return 'g2048_best_' + size
}

function loadBest(size) {
  return Number(wx.getStorageSync(bestKey(size)) || 0)
}

function saveBest(size, v) {
  wx.setStorageSync(bestKey(size), v)
}

function loadHistory() {
  return wx.getStorageSync('g2048_history') || []
}

function pad(n) {
  return n < 10 ? '0' + n : '' + n
}

function stamp() {
  const d = new Date()
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes())
}

// 记一局成绩（0 分的不记）；只留分数最高的 50 局
function addRecord(rec) {
  if (!rec.score) return
  const list = loadHistory()
  list.push(Object.assign({ at: stamp() }, rec))
  list.sort((a, b) => b.score - a.score)
  wx.setStorageSync('g2048_history', list.slice(0, 50))
}

function clearHistory() {
  wx.removeStorageSync('g2048_history')
}

module.exports = { SIZES, THEMES, loadSettings, saveSettings, theme, loadBest, saveBest, loadHistory, addRecord, clearHistory }
