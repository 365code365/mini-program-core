// 计算器：iOS 计算器式的状态机（当前输入 / 上一个数 / 待执行的运算符）
const KEYS = [
  { k: 'ac', t: 'AC', cls: 'fn' }, { k: 'neg', t: '±', cls: 'fn' }, { k: 'pct', t: '%', cls: 'fn' }, { k: '/', t: '÷', cls: 'op' },
  { k: '7', t: '7' }, { k: '8', t: '8' }, { k: '9', t: '9' }, { k: '*', t: '×', cls: 'op' },
  { k: '4', t: '4' }, { k: '5', t: '5' }, { k: '6', t: '6' }, { k: '-', t: '－', cls: 'op' },
  { k: '1', t: '1' }, { k: '2', t: '2' }, { k: '3', t: '3' }, { k: '+', t: '+', cls: 'op' },
  { k: '0', t: '0', cls: 'zero' }, { k: '.', t: '.' }, { k: '=', t: '=', cls: 'eq' }
]
const SYM = { '+': '+', '-': '－', '*': '×', '/': '÷' }

// 去掉浮点尾巴：0.1+0.2 → 0.3
function clean(n) {
  if (!isFinite(n)) return n
  return parseFloat(n.toPrecision(12))
}
function show(n) {
  if (typeof n === 'string') return n
  if (!isFinite(n)) return '错误'
  const s = String(clean(n))
  if (s.length > 12) return clean(n).toExponential(6).replace('e+', 'e')
  return s
}
function apply(a, op, b) {
  if (op === '+') return a + b
  if (op === '-') return a - b
  if (op === '*') return a * b
  if (op === '/') return b === 0 ? NaN : a / b
  return b
}

Page({
  data: { keys: KEYS, display: '0', expr: '', op: '', history: [], small: false },
  onLoad() {
    this._cur = '0'; this._prev = null; this._op = null; this._fresh = false
    this.setData({ history: wx.getStorageSync('tb_calc_history') || [] })
  },
  paint(expr) {
    const d = this._cur
    this.setData({ display: d, small: d.length > 9, op: this._fresh ? '' : (this._op || ''), expr: expr !== undefined ? expr : this.data.expr })
  },
  onPad(e) {
    // String()：引擎把看起来像数字的 dataset 值转成了 number（微信里是字符串）
    const k = String(e.currentTarget.dataset.k)
    if (this._cur === '错误' && k !== 'ac') this._cur = '0'
    if (/^[0-9]$/.test(k)) {
      if (this._fresh) { this._cur = '0'; this._fresh = false }
      if (this._cur.replace(/[-.]/g, '').length >= 12) return
      this._cur = this._cur === '0' ? k : this._cur === '-0' ? '-' + k : this._cur + k
      return this.paint()
    }
    if (k === '.') {
      if (this._fresh) { this._cur = '0'; this._fresh = false }
      if (this._cur.indexOf('.') < 0) this._cur += '.'
      return this.paint()
    }
    if (k === 'ac') { this._cur = '0'; this._prev = null; this._op = null; this._fresh = false; return this.paint('') }
    if (k === 'neg') {
      this._cur = this._cur.charAt(0) === '-' ? this._cur.slice(1) : '-' + this._cur
      if (this._fresh) this._fresh = false
      return this.paint()
    }
    if (k === 'pct') {
      // 有待执行的加减时，按「上一个数的百分之几」算（100+10% = 110），否则直接除 100
      const v = parseFloat(this._cur)
      const r = (this._prev !== null && (this._op === '+' || this._op === '-')) ? this._prev * v / 100 : v / 100
      this._cur = show(r); this._fresh = false
      return this.paint()
    }
    if (SYM[k]) {
      const v = parseFloat(this._cur)
      if (this._prev !== null && this._op && !this._fresh) {
        const r = apply(this._prev, this._op, v)
        this._cur = show(r); this._prev = isFinite(r) ? clean(r) : null
      } else {
        this._prev = v
      }
      this._op = k; this._fresh = true
      this.paint(show(this._prev === null ? 0 : this._prev) + ' ' + SYM[k])
      this.setData({ op: k })
      return
    }
    if (k === '=') {
      if (this._prev === null || !this._op) return
      const b = parseFloat(this._cur)
      const r = apply(this._prev, this._op, b)
      const expr = show(this._prev) + ' ' + SYM[this._op] + ' ' + show(b)
      this._cur = show(r)
      this._prev = null; this._op = null; this._fresh = true
      this.paint(expr + ' =')
      if (this._cur !== '错误') {
        const history = [{ e: expr, r: this._cur }].concat(this.data.history).slice(0, 8)
        wx.setStorageSync('tb_calc_history', history)
        this.setData({ history })
      }
    }
  },
  onHistory(e) {
    this._cur = String(e.currentTarget.dataset.r); this._prev = null; this._op = null; this._fresh = true
    this.paint('')
  },
  onClear() {
    wx.showModal({
      title: '清空计算记录？', confirmColor: '#4056ee',
      success: (res) => { if (res.confirm) { wx.removeStorageSync('tb_calc_history'); this.setData({ history: [] }) } }
    })
  }
})
