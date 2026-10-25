// 笔记、标签、设置：全部落在本地 storage（不联网）
const KEY_NOTES = 'notes_list'
const KEY_TAGS = 'notes_tags'
const KEY_SETTINGS = 'notes_settings'

const COLORS = [
  { key: 'white', name: '素白', hex: '#ffffff' },
  { key: 'yellow', name: '暖黄', hex: '#fff4c2' },
  { key: 'green', name: '薄荷', hex: '#dff5e3' },
  { key: 'blue', name: '天青', hex: '#dcecff' },
  { key: 'pink', name: '樱粉', hex: '#ffe1e6' }
]
const SORTS = ['按修改时间', '按创建时间', '按标题']
const DEFAULT_SETTINGS = { sort: 0, preview: true }

function pad(n) {
  return n < 10 ? '0' + n : '' + n
}
// 「10-25 14:03」
function fmtTime(ts) {
  const d = new Date(ts)
  return pad(d.getMonth() + 1) + '-' + pad(d.getDate()) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes())
}
function colorHex(key) {
  const c = COLORS.find(i => i.key === key)
  return c ? c.hex : '#ffffff'
}

function seedTags() {
  return [
    { id: 't1', name: '工作' },
    { id: 't2', name: '生活' },
    { id: 't3', name: '读书' },
    { id: 't4', name: '旅行' },
    { id: 't5', name: '灵感' }
  ]
}
function seedNotes() {
  const now = Date.now()
  const H = 3600 * 1000
  // created / updated 用相对「现在」的偏移，排序结果与日期无关
  const raw = [
    [1, '周末徒步路线', '九溪烟树 → 龙井村 → 十里琅珰，全程约 12 公里，记得带护膝和两瓶水。', ['t4'], 'yellow', true, 90, 2],
    [2, '读书摘录：《置身事内》', '地方政府的“土地财政”本质上是一种融资模式，城投公司是关键角色。', ['t3'], 'blue', true, 200, 30],
    [3, '产品周会要点', '1. 新版首页灰度 10%\n2. 留存看 7 日\n3. 下周三前给出埋点方案', ['t1'], 'white', false, 50, 5],
    [4, '番茄炒蛋的正确做法', '蛋液加少许盐和水，大火快炒盛出；番茄去皮炒出沙，再回锅蛋。', ['t2'], 'pink', false, 120, 60],
    [5, '年度旅行清单', '大理、阿勒泰、京都赏枫、冰岛极光（存钱中）。', ['t4', 't5'], 'green', false, 400, 10],
    [6, '灵感：桌面收纳盒', '用磁吸模块拼接，带无线充电位，木纹 + 磨砂铝两种材质。', ['t5'], 'yellow', false, 30, 20],
    [7, 'Rust 所有权笔记', '借用检查器：同一时刻要么一个可变引用，要么多个不可变引用。', ['t1', 't5'], 'white', false, 300, 1],
    [8, '健身计划 第 3 周', '周一胸背、周三腿、周五肩臂，有氧每次 20 分钟。', ['t2'], 'green', false, 160, 40]
  ]
  const list = raw.map(r => ({
    id: r[0], title: r[1], body: r[2], tags: r[3], color: r[4], pinned: r[5],
    created: now - r[6] * H, updated: now - r[7] * H, deleted: false
  }))
  list.push({ id: 9, title: '旧的购物清单', body: '牛奶、鸡蛋、全麦面包', tags: ['t2'], color: 'white', pinned: false, created: now - 500 * H, updated: now - 300 * H, deleted: true })
  list.push({ id: 10, title: '过期的会议纪要', body: '已同步到飞书文档', tags: ['t1'], color: 'white', pinned: false, created: now - 520 * H, updated: now - 310 * H, deleted: true })
  return list
}

function loadNotes() {
  let list = wx.getStorageSync(KEY_NOTES)
  if (!list || !list.length) {
    // 第一次打开才写种子数据；用户把笔记全删光之后存的是 []，不能再把种子写回来（_inited 标记）
    if (wx.getStorageSync(KEY_NOTES + '_inited')) return []
    list = seedNotes()
    saveNotes(list)
  }
  return list
}
function saveNotes(list) {
  wx.setStorageSync(KEY_NOTES, list)
  wx.setStorageSync(KEY_NOTES + '_inited', true)
}
function loadTags() {
  let tags = wx.getStorageSync(KEY_TAGS)
  if (!tags) {
    tags = seedTags()
    wx.setStorageSync(KEY_TAGS, tags)
  }
  return tags
}
function saveTags(tags) {
  wx.setStorageSync(KEY_TAGS, tags)
}
function loadSettings() {
  return Object.assign({}, DEFAULT_SETTINGS, wx.getStorageSync(KEY_SETTINGS) || {})
}
function saveSettings(s) {
  wx.setStorageSync(KEY_SETTINGS, s)
}
function nextId(list) {
  return list.reduce((m, i) => Math.max(m, i.id), 0) + 1
}
// 排序：标题按码位比较（不依赖 Intl，各端结果一致）
function sortNotes(list, sort) {
  const by = [
    (a, b) => b.updated - a.updated,
    (a, b) => b.created - a.created,
    (a, b) => (a.title < b.title ? -1 : a.title > b.title ? 1 : 0)
  ][sort] || ((a, b) => b.updated - a.updated)
  return list.slice().sort(by)
}
// 正文字数：去掉空白后的字符数
function wordCount(text) {
  return String(text || '').replace(/\s/g, '').length
}

module.exports = {
  COLORS, SORTS, KEY_NOTES, KEY_TAGS, KEY_SETTINGS,
  fmtTime, colorHex, loadNotes, saveNotes, loadTags, saveTags, loadSettings, saveSettings,
  nextId, sortNotes, wordCount
}
