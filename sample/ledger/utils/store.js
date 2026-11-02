// 账单数据层：所有记录都存在 storage（ledger_records），预算存在 ledger_budget（默认账本）/ ledger_books[i].budget
// 账本：ledger_books（[{id, name, color, budget}]）+ 当前账本 ledger_book；记录的 book 字段缺省算默认账本 b1。
// 页面上的「本月」「全部」都只看当前账本（all()），读写原始列表用 raw()。
// 自定义分类：ledger_cats_custom（{out: [...], in: [...]}），排在内置分类之后、「其他」之前。
const KEY = 'ledger_records'
const BKEY = 'ledger_budget'
const INIT_KEY = 'ledger_inited'
const BOOKS_KEY = 'ledger_books'
const CUR_KEY = 'ledger_book'
const CUSTOM_KEY = 'ledger_cats_custom'
const REMIND_KEY = 'ledger_reminder'
const CATS = {
  out: [
    { name: '餐饮', ch: '餐', color: '#ff8a4c' },
    { name: '交通', ch: '行', color: '#4c9bff' },
    { name: '购物', ch: '购', color: '#ff5f8a' },
    { name: '娱乐', ch: '娱', color: '#9b6bff' },
    { name: '居住', ch: '住', color: '#2fbf8f' },
    { name: '通讯', ch: '讯', color: '#36c2d9' },
    { name: '医疗', ch: '医', color: '#f25b5b' },
    { name: '教育', ch: '学', color: '#f5a623' },
    { name: '人情', ch: '礼', color: '#e8657b' },
    { name: '其他', ch: '其', color: '#8f96a3' }
  ],
  in: [
    { name: '工资', ch: '薪', color: '#f57c34' },
    { name: '奖金', ch: '奖', color: '#f5a623' },
    { name: '理财', ch: '财', color: '#2fbf8f' },
    { name: '兼职', ch: '兼', color: '#4c9bff' },
    { name: '红包', ch: '红', color: '#f25b5b' },
    { name: '其他', ch: '其', color: '#8f96a3' }
  ]
}
const PALETTE = ['#ff8a4c', '#4c9bff', '#2fbf8f', '#9b6bff', '#ff5f8a', '#f5a623', '#36c2d9', '#5c6066']
const DEFAULT_BOOK = { id: 'b1', name: '日常账本', color: '#f57c34' }
const WEEK = ['星期日', '星期一', '星期二', '星期三', '星期四', '星期五', '星期六']
function pad(n) { return n < 10 ? '0' + n : '' + n }
function fmtDate(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) }
function today() { return fmtDate(new Date()) }
function parseDate(s) {
  const p = s.split('-')
  return new Date(+p[0], +p[1] - 1, +p[2])
}
function money(v) { return (Math.round(v * 100) / 100).toFixed(2) }
// 带符号的金额：-¥12.00 而不是 ¥-12.00
function yuan(v) { return (v < 0 ? '-¥' : '¥') + money(Math.abs(v)) }

// ---------- 分类 ----------
function custom() {
  const c = wx.getStorageSync(CUSTOM_KEY)
  return { out: (c && Array.isArray(c.out)) ? c.out : [], in: (c && Array.isArray(c.in)) ? c.in : [] }
}
function cats(type) {
  const base = CATS[type] || CATS.out
  const extra = custom()[type === 'in' ? 'in' : 'out'].map(c => Object.assign({ custom: true }, c))
  return base.slice(0, -1).concat(extra, base.slice(-1))
}
function catOf(type, name) {
  const list = cats(type)
  for (let i = 0; i < list.length; i++) if (list[i].name === name) return list[i]
  return list[list.length - 1]
}
// 返回错误文案；成功返回 ''
function addCat(type, name, color) {
  name = (name || '').trim()
  if (!name) return '请输入分类名称'
  if (name.length > 4) return '分类名称最多 4 个字'
  if (cats(type).some(c => c.name === name)) return '已有同名分类'
  const c = custom()
  c[type].push({ name: name, ch: name.charAt(0), color: color || PALETTE[c[type].length % PALETTE.length] })
  wx.setStorageSync(CUSTOM_KEY, c)
  return ''
}
// 删掉自定义分类：这个分类下的账单（所有账本）归入「其他」，返回挪动的笔数
function removeCat(type, name) {
  const c = custom()
  c[type] = c[type].filter(x => x.name !== name)
  wx.setStorageSync(CUSTOM_KEY, c)
  let moved = 0
  const list = raw()
  list.forEach(r => { if (r.type === type && r.cat === name) { r.cat = '其他'; moved++ } })
  if (moved) saveAll(list)
  return moved
}
function catCount(type, name) { return raw().filter(r => r.type === type && r.cat === name).length }

// ---------- 账本 ----------
function books() {
  const b = wx.getStorageSync(BOOKS_KEY)
  return Array.isArray(b) && b.length ? b : [Object.assign({}, DEFAULT_BOOK)]
}
function saveBooks(list) { wx.setStorageSync(BOOKS_KEY, list) }
function curBookId() {
  const id = wx.getStorageSync(CUR_KEY)
  const list = books()
  return list.some(b => b.id === id) ? id : list[0].id
}
function curBook() {
  const id = curBookId()
  return books().filter(b => b.id === id)[0]
}
function setCurBook(id) { wx.setStorageSync(CUR_KEY, id) }
function bookOf(r) { return r.book || 'b1' }
function saveBook(id, name, color) {
  name = (name || '').trim()
  if (!name) return '请输入账本名称'
  if (name.length > 8) return '账本名称最多 8 个字'
  const list = books()
  if (list.some(b => b.name === name && b.id !== id)) return '已有同名账本'
  if (id) list.forEach(b => { if (b.id === id) { b.name = name; b.color = color || b.color } })
  else list.push({ id: 'b' + Date.now(), name: name, color: color || PALETTE[list.length % PALETTE.length], budget: 3000 })
  saveBooks(list)
  return ''
}
// 删账本：连同它的账单一起删；当前账本被删就切回第一个
function removeBook(id) {
  if (id === 'b1') return
  saveBooks(books().filter(b => b.id !== id))
  saveAll(raw().filter(r => bookOf(r) !== id))
  if (wx.getStorageSync(CUR_KEY) === id) setCurBook('b1')
}
function bookCount(id) { return raw().filter(r => bookOf(r) === id).length }

// ---------- 示例数据 ----------
// 以今天为基准往前铺 45 天，伪随机但可复现
function seed() {
  let s = 20261021
  const rnd = () => { s = (s * 1103515245 + 12345) % 2147483648; return s / 2147483648 }
  const outs = [
    ['餐饮', ['瑞幸咖啡 生椰拿铁', '午饭 黄焖鸡米饭', '盒马 晚饭食材', '海底捞 朋友聚餐', '早餐 包子豆浆', '美团外卖 麻辣香锅'], 12, 180],
    ['交通', ['地铁通勤', '滴滴快车 回家', '共享单车月卡', '加油 92#'], 4, 320],
    ['购物', ['京东 洗衣凝珠', '优衣库 卫衣', '淘宝 数据线', '屈臣氏 洗面奶'], 25, 299],
    ['娱乐', ['电影《长安三万里》', 'KTV 欢唱三小时', 'Steam 游戏'], 39, 168],
    ['通讯', ['话费充值'], 50, 100],
    ['医疗', ['药房 感冒灵颗粒'], 18, 66],
    ['人情', ['同事婚礼份子'], 500, 800]
  ]
  const recs = []
  const now = new Date()
  let id = 1
  for (let back = 0; back < 45; back++) {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() - back)
    const ds = fmtDate(d)
    const n = 1 + Math.floor(rnd() * 3)
    for (let k = 0; k < n; k++) {
      let pick = Math.floor(rnd() * 10)
      if (pick > 6) pick = 0
      if (pick === 6 && rnd() < 0.85) pick = 1
      const o = outs[pick]
      const note = o[1][Math.floor(rnd() * o[1].length)]
      const amt = o[2] + Math.round(rnd() * (o[3] - o[2]) * 10) / 10
      recs.push({ id: 'r' + (id++), type: 'out', cat: o[0], amount: amt, note: note, date: ds, ts: d.getTime() + k })
    }
    if (d.getDate() === 10) recs.push({ id: 'r' + (id++), type: 'in', cat: '工资', amount: 15800, note: '10号发薪', date: ds, ts: d.getTime() + 9 })
    if (d.getDate() === 1) recs.push({ id: 'r' + (id++), type: 'out', cat: '居住', amount: 3200, note: '房租 · 月付', date: ds, ts: d.getTime() + 8 })
    if (back === 3) recs.push({ id: 'r' + (id++), type: 'in', cat: '理财', amount: 126.38, note: '余额宝收益', date: ds, ts: d.getTime() + 7 })
    if (back === 12) recs.push({ id: 'r' + (id++), type: 'in', cat: '兼职', amount: 800, note: '周末设计稿', date: ds, ts: d.getTime() + 6 })
  }
  // 第二个账本：最近一次短途旅行
  const trip = [
    [1, '交通', '高铁 杭州东 → 黄山北', 236.5],
    [1, '居住', '民宿 两晚', 688],
    [2, '餐饮', '徽菜馆 臭鳜鱼', 168],
    [2, '娱乐', '黄山风景区门票', 190],
    [3, '购物', '伴手礼 毛峰茶叶', 128]
  ]
  trip.forEach(t => {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() - t[0])
    recs.push({ id: 'r' + (id++), book: 'b2', type: 'out', cat: t[1], amount: t[3], note: t[2], date: fmtDate(d), ts: d.getTime() + id })
  })
  return recs
}
function ensureSeed() {
  if (!wx.getStorageSync(INIT_KEY)) {
    wx.setStorageSync(KEY, seed())
    wx.setStorageSync(BKEY, 8000)
    saveBooks([Object.assign({}, DEFAULT_BOOK), { id: 'b2', name: '旅行账本', color: '#2fbf8f', budget: 2000 }])
    wx.setStorageSync(CUR_KEY, 'b1')
    wx.setStorageSync(INIT_KEY, 1)
  }
}

// ---------- 账单 ----------
function raw() {
  const r = wx.getStorageSync(KEY)
  return Array.isArray(r) ? r : []
}
// 当前账本的全部账单
function all() {
  const id = curBookId()
  return raw().filter(r => bookOf(r) === id)
}
function saveAll(list) { wx.setStorageSync(KEY, list) }
function get(id) {
  const list = raw()
  for (let i = 0; i < list.length; i++) if (String(list[i].id) === String(id)) return list[i]
  return null
}
function upsert(rec) {
  const list = raw()
  if (!rec.id) {
    rec.id = 'u' + Date.now()
    rec.ts = Date.now()
    rec.book = curBookId()
    list.push(rec)
  } else {
    for (let i = 0; i < list.length; i++) if (list[i].id === rec.id) { list[i] = Object.assign({}, list[i], rec) }
  }
  saveAll(list)
  return rec.id
}
function remove(id) { saveAll(raw().filter(r => String(r.id) !== String(id))) }
// 清空当前账本
function clear() {
  const id = curBookId()
  saveAll(raw().filter(r => bookOf(r) !== id))
}
// 预算（当前账本）：默认账本沿用 ledger_budget，其余账本记在账本上
function getBudget() {
  const b = curBook()
  const v = b.id === 'b1' ? wx.getStorageSync(BKEY) : b.budget
  return typeof v === 'number' && v > 0 ? v : 8000
}
function setBudget(v) {
  const id = curBookId()
  if (id === 'b1') { wx.setStorageSync(BKEY, v); return }
  const list = books()
  list.forEach(b => { if (b.id === id) b.budget = v })
  saveBooks(list)
}
// 某年某月（month 1~12）的记录（当前账本），按时间倒序
function monthRecords(y, m) {
  const prefix = y + '-' + pad(m) + '-'
  return all().filter(r => r.date.indexOf(prefix) === 0).sort(byTime)
}
function byTime(a, b) { return b.date > a.date ? 1 : b.date < a.date ? -1 : b.ts - a.ts }
function sum(list, type) {
  let t = 0
  list.forEach(r => { if (r.type === type) t += r.amount })
  return Math.round(t * 100) / 100
}
function decorate(r) {
  const c = catOf(r.type, r.cat)
  return Object.assign({}, r, { ch: c.ch, color: c.color, amountText: (r.type === 'out' ? '-' : '+') + money(r.amount) })
}
function dayLabel(ds) {
  const d = parseDate(ds)
  const t = today()
  const y = fmtDate(new Date(Date.now() - 86400000))
  const md = (d.getMonth() + 1) + '月' + d.getDate() + '日'
  if (ds === t) return md + ' 今天'
  if (ds === y) return md + ' 昨天'
  return md + ' ' + WEEK[d.getDay()]
}
// 当月支出占预算的比例（0~∞）
function monthUsage() {
  const now = new Date()
  return sum(monthRecords(now.getFullYear(), now.getMonth() + 1), 'out') / getBudget()
}

// ---------- 提醒 ----------
function reminder() {
  const r = wx.getStorageSync(REMIND_KEY)
  return r && typeof r === 'object' ? r : { on: true, time: '21:00', days: 'everyday' }
}
function setReminder(r) { wx.setStorageSync(REMIND_KEY, r) }

module.exports = {
  CATS, PALETTE, WEEK, pad, fmtDate, today, parseDate, money, yuan,
  cats, catOf, addCat, removeCat, catCount,
  books, curBook, curBookId, setCurBook, saveBook, removeBook, bookCount,
  ensureSeed, raw, all, get, upsert, remove, clear, getBudget, setBudget,
  monthRecords, byTime, sum, decorate, dayLabel, monthUsage, reminder, setReminder
}
