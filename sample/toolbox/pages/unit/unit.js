// 单位换算：长度 / 重量按「换算到基准单位的系数」，温度单独用公式
const GROUPS = [
  { key: 'len', name: '长度', units: [
    { n: '千米', s: 'km', f: 1000 }, { n: '米', s: 'm', f: 1 }, { n: '厘米', s: 'cm', f: 0.01 }, { n: '毫米', s: 'mm', f: 0.001 },
    { n: '英里', s: 'mi', f: 1609.344 }, { n: '英尺', s: 'ft', f: 0.3048 }, { n: '英寸', s: 'in', f: 0.0254 }, { n: '市里', s: '里', f: 500 }, { n: '市尺', s: '尺', f: 1 / 3 }
  ] },
  { key: 'wt', name: '重量', units: [
    { n: '吨', s: 't', f: 1000 }, { n: '千克', s: 'kg', f: 1 }, { n: '克', s: 'g', f: 0.001 }, { n: '斤', s: '斤', f: 0.5 }, { n: '两', s: '两', f: 0.05 },
    { n: '磅', s: 'lb', f: 0.45359237 }, { n: '盎司', s: 'oz', f: 0.028349523 }, { n: '克拉', s: 'ct', f: 0.0002 }
  ] },
  { key: 'tmp', name: '温度', units: [
    { n: '摄氏度', s: '℃' }, { n: '华氏度', s: '℉' }, { n: '开尔文', s: 'K' }
  ] }
]

function toC(v, s) { return s === '℃' ? v : s === '℉' ? (v - 32) * 5 / 9 : v - 273.15 }
function fromC(c, s) { return s === '℃' ? c : s === '℉' ? c * 9 / 5 + 32 : c + 273.15 }
function fmt(v) {
  if (!isFinite(v)) return '-'
  const a = Math.abs(v)
  if (a !== 0 && (a >= 1e9 || a < 1e-4)) return v.toExponential(4)
  return String(parseFloat(v.toPrecision(8)))
}

Page({
  data: { groups: GROUPS.map(g => g.name), gi: 0, unitNames: [], ui: 1, value: '1', from: null, list: [] },
  onLoad() { this.switchGroup(0) },
  switchGroup(gi) {
    const g = GROUPS[gi]
    const ui = gi === 2 ? 0 : 1
    this.setData({ gi, ui, unitNames: g.units.map(u => u.n + ' (' + u.s + ')'), value: gi === 2 ? '36.5' : '1' })
    this.calc()
  },
  calc() {
    const g = GROUPS[this.data.gi]
    const from = g.units[this.data.ui]
    const v = parseFloat(this.data.value)
    const ok = !isNaN(v)
    const list = g.units.filter((u, i) => i !== this.data.ui).map(u => {
      let r
      if (g.key === 'tmp') r = fromC(toC(v, from.s), u.s)
      else r = v * from.f / u.f
      return { n: u.n, s: u.s, v: ok ? fmt(r) : '-' }
    })
    this.setData({ from, list })
  },
  onTab(e) { const gi = +e.currentTarget.dataset.i; if (gi !== this.data.gi) this.switchGroup(gi) },
  onPick(e) { this.setData({ ui: +e.detail.value }); this.calc() },
  onInput(e) { this.setData({ value: e.detail.value }); this.calc() },
  onRow(e) {
    const g = GROUPS[this.data.gi]
    const i = g.units.findIndex(u => u.s === e.currentTarget.dataset.s)
    const v = String(e.currentTarget.dataset.v)
    this.setData({ ui: i, value: v === '-' ? '1' : v }); this.calc()
  }
})
