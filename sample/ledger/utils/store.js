// 账单数据层：所有记录都存在 storage（ledger_records），预算存在 ledger_budget
const KEY = 'ledger_records'
const BKEY = 'ledger_budget'
const INIT_KEY = 'ledger_inited'

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

const WEEK = ['星期日', '星期一', '星期二', '星期三', '星期四', '星期五', '星期六']

function pad(n) { return n < 10 ? '0' + n : '' + n }
function fmtDate(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) }
function today() { return fmtDate(new Date()) }
function parseDate(s) {
  const p = s.split('-')
  return new Date(+p[0], +p[1] - 1, +p[2])
}
function money(v) { return (Math.round(v * 100) / 100).toFixed(2) }

function catOf(type, name) {
  const list = CATS[type] || CATS.out
  for (let i = 0; i < list.length; i++) if (list[i].name === name) return list[i]
  return list[list.length - 1]
}

// 示例数据：以今天为基准往前铺 45 天，伪随机但可复现
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
  return recs
}

function ensureSeed() {
  if (!wx.getStorageSync(INIT_KEY)) {
    wx.setStorageSync(KEY, seed())
    wx.setStorageSync(BKEY, 8000)
    wx.setStorageSync(INIT_KEY, 1)
  }
}

function all() {
  const r = wx.getStorageSync(KEY)
  return Array.isArray(r) ? r : []
}
function saveAll(list) { wx.setStorageSync(KEY, list) }
function get(id) {
  const list = all()
  for (let i = 0; i < list.length; i++) if (list[i].id === id) return list[i]
  return null
}
function upsert(rec) {
  const list = all()
  if (!rec.id) {
    rec.id = 'u' + Date.now()
    rec.ts = Date.now()
    list.push(rec)
  } else {
    for (let i = 0; i < list.length; i++) if (list[i].id === rec.id) { list[i] = Object.assign({}, list[i], rec) }
  }
  saveAll(list)
  return rec.id
}
function remove(id) { saveAll(all().filter(r => r.id !== id)) }
function clear() { saveAll([]) }
function getBudget() { return wx.getStorageSync(BKEY) || 8000 }
function setBudget(v) { wx.setStorageSync(BKEY, v) }

// 某年某月（month 1~12）的记录，按时间倒序
function monthRecords(y, m) {
  const prefix = y + '-' + pad(m) + '-'
  return all().filter(r => r.date.indexOf(prefix) === 0).sort((a, b) => (b.date > a.date ? 1 : b.date < a.date ? -1 : b.ts - a.ts))
}
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

module.exports = {
  CATS, WEEK, pad, fmtDate, today, parseDate, money, catOf,
  ensureSeed, all, get, upsert, remove, clear, getBudget, setBudget,
  monthRecords, sum, decorate, dayLabel
}
