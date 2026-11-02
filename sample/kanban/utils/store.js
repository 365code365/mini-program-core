// 看板数据：列、成员、优先级是常量；卡片落在 storage（kb_cards）
const KEY = 'kb_cards'
const COLS = [
  { key: 'todo', name: '待办', color: '#8c8c8c' },
  { key: 'doing', name: '进行中', color: '#2f54eb' },
  { key: 'done', name: '已完成', color: '#52c41a' }
]
const PRIOS = [
  { name: '低', color: '#8c8c8c' },
  { name: '中', color: '#1890ff' },
  { name: '高', color: '#fa8c16' },
  { name: '紧急', color: '#f5222d' }
]
const MEMBERS = [
  { id: 'm1', name: '林晓', role: '产品经理', color: '#eb2f96' },
  { id: 'm2', name: '陈默', role: '前端工程师', color: '#2f54eb' },
  { id: 'm3', name: '王蕾', role: '视觉设计', color: '#fa8c16' },
  { id: 'm4', name: '赵磊', role: '后端工程师', color: '#13c2c2' },
  { id: 'm5', name: '孙悦', role: '测试工程师', color: '#722ed1' }
]

function pad(n) {
  return n < 10 ? '0' + n : '' + n
}
function fmt(d) {
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate())
}
// 今天 ± n 天（本地时区）。截止日期都按它生成，统计结果与哪天跑无关
function dayOffset(n) {
  const d = new Date()
  d.setDate(d.getDate() + n)
  return fmt(d)
}
function today() {
  return dayOffset(0)
}
function isOverdue(c) {
  return c.col !== 'done' && !!c.due && c.due < today()
}
function dueLabel(due) {
  if (!due) return '无截止'
  if (due === today()) return '今天'
  if (due === dayOffset(1)) return '明天'
  if (due === dayOffset(-1)) return '昨天'
  return due.slice(5)
}
function member(id) {
  return MEMBERS.find(m => m.id === id) || { id: '', name: '未分配', role: '', color: '#bfbfbf' }
}

function seed() {
  const s = (text, done) => ({ text, done })
  const raw = [
    ['首页改版需求评审', '整理三版方案的优缺点，周会上过一遍', 'todo', 2, 'm1', 3, [s('收集竞品截图', true), s('写评审文档', false), s('约评审会', false)]],
    ['登录页埋点方案', '注册转化漏斗：曝光 → 输入手机号 → 验证码 → 完成', 'todo', 1, 'm4', -2, [s('梳理事件', true), s('评审字段', false)]],
    ['图标库整理', '统一 24px 栅格，导出 SVG', 'todo', 0, 'm3', 7, []],
    ['订单列表虚拟滚动', '1 万条数据首屏 < 300ms', 'doing', 3, 'm2', -1, [s('技术预研', true), s('实现', true), s('压测', false), s('灰度', false)]],
    ['支付回调重试', '指数退避，最多 5 次，失败告警', 'doing', 2, 'm4', 1, [s('方案', true), s('开发', false)]],
    ['新版插画', '空态、404、网络错误三张', 'doing', 1, 'm3', 0, [s('草图', true), s('上色', false), s('切图', false)]],
    ['回归测试用例补齐', '覆盖下单、退款、优惠券叠加', 'doing', 1, 'm5', 2, []],
    ['发布 2.3.0', '应用商店审核 + 灰度 10%', 'done', 2, 'm1', -3, [s('提审', true), s('灰度', true)]],
    ['性能基线报告', '冷启动、首屏、内存三项', 'done', 1, 'm5', -5, []],
    ['修复 iOS 键盘遮挡', '输入框聚焦时页面上推', 'done', 3, 'm2', -1, [s('复现', true), s('修复', true), s('回归', true)]]
  ]
  return raw.map((r, i) => ({
    id: i + 1, title: r[0], desc: r[1], col: r[2], prio: r[3], owner: r[4], due: dayOffset(r[5]),
    subs: r[6].map((x, k) => ({ id: k + 1, text: x.text, done: x.done }))
  }))
}

function load() {
  let list = wx.getStorageSync(KEY)
  if (!Array.isArray(list)) {
    list = seed()
    wx.setStorageSync(KEY, list)
  }
  return list
}
function save(list) {
  wx.setStorageSync(KEY, list)
}
function find(id) {
  return load().find(c => c.id === id) || null
}
// 改一张卡：fn 就地修改，改完落盘
function update(id, fn) {
  const list = load()
  const c = list.find(i => i.id === id)
  if (!c) return null
  fn(c)
  save(list)
  return c
}
function nextId(list) {
  return list.reduce((m, c) => Math.max(m, c.id), 0) + 1
}
function progress(c) {
  const n = c.subs.length
  const d = c.subs.filter(s => s.done).length
  return { done: d, total: n, pct: n ? Math.round(d * 100 / n) : 0 }
}
// 列表展示用的卡片
function view(c) {
  const p = progress(c)
  const m = member(c.owner)
  return {
    id: c.id, title: c.title, col: c.col,
    prioName: PRIOS[c.prio].name, prioColor: PRIOS[c.prio].color,
    owner: m.name, ownerColor: m.color, ownerInitial: m.name.slice(-1),
    due: c.due, dueText: dueLabel(c.due), overdue: isOverdue(c),
    subText: p.total ? p.done + '/' + p.total : '', pct: p.pct
  }
}

// ───── 团队动态（kb_log）与评论：「我」固定是林晓（产品经理，看板的创建人） ─────
const LOG_KEY = 'kb_log'
const LOG_MAX = 200
const ME = 'm1'
const LOG_TYPES = [
  { key: 'create', name: '新建' },
  { key: 'move', name: '移动' },
  { key: 'edit', name: '修改' },
  { key: 'comment', name: '评论' },
  { key: 'delete', name: '删除' }
]
function seedLog() {
  const H = 3600 * 1000
  const now = Date.now()
  const raw = [
    [50, 'm1', 'create', 1, '首页改版需求评审', '新建了卡片'],
    [30, 'm4', 'edit', 2, '登录页埋点方案', '勾选了子任务「梳理事件」'],
    [26, 'm2', 'move', 4, '订单列表虚拟滚动', '从「待办」移到「进行中」'],
    [20, 'm1', 'move', 8, '发布 2.3.0', '从「进行中」移到「已完成」'],
    [6, 'm3', 'comment', 6, '新版插画', '评论：草图已发群里，等确认配色'],
    [2, 'm2', 'move', 10, '修复 iOS 键盘遮挡', '从「进行中」移到「已完成」']
  ]
  return raw.map((r, i) => ({ id: i + 1, at: now - r[0] * H, who: r[1], type: r[2], cardId: r[3], title: r[4], text: r[5] }))
}
function loadLog() {
  let list = wx.getStorageSync(LOG_KEY)
  if (!Array.isArray(list)) {
    list = seedLog()
    wx.setStorageSync(LOG_KEY, list)
  }
  return list
}
// 记一条动态（最新的在最后，最多留 LOG_MAX 条）
function log(type, card, text) {
  const list = loadLog()
  const id = list.reduce((m, i) => Math.max(m, i.id), 0) + 1
  list.push({ id, at: Date.now(), who: ME, type, cardId: card.id, title: card.title, text })
  wx.setStorageSync(LOG_KEY, list.slice(-LOG_MAX))
}
function colName(key) {
  const c = COLS.find(i => i.key === key)
  return c ? c.name : key
}
// 「10-25 14:03」
function timeText(ts) {
  const d = new Date(ts)
  return pad(d.getMonth() + 1) + '-' + pad(d.getDate()) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes())
}
module.exports = { KEY, COLS, LOG_KEY, LOG_TYPES, ME, loadLog, log, colName, timeText, PRIOS, MEMBERS, fmt, dayOffset, today, isOverdue, dueLabel, member, load, save, find, update, nextId, progress, view }
