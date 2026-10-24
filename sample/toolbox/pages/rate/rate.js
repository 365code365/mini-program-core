// 内置汇率表：1 单位外币折合人民币（参考价，离线使用）
const CUR = [
  { code: 'CNY', name: '人民币', sym: '¥', flag: '中', color: '#e8453c', cny: 1 },
  { code: 'USD', name: '美元', sym: '$', flag: '美', color: '#3a5bd9', cny: 7.1236 },
  { code: 'EUR', name: '欧元', sym: '€', flag: '欧', color: '#2f6fd6', cny: 7.7418 },
  { code: 'JPY', name: '日元', sym: '¥', flag: '日', color: '#d93a4a', cny: 0.047812 },
  { code: 'HKD', name: '港币', sym: 'HK$', flag: '港', color: '#d94a3a', cny: 0.91547 },
  { code: 'GBP', name: '英镑', sym: '£', flag: '英', color: '#24418f', cny: 9.0563 },
  { code: 'KRW', name: '韩元', sym: '₩', flag: '韩', color: '#3c63c9', cny: 0.005132 },
  { code: 'AUD', name: '澳元', sym: 'A$', flag: '澳', color: '#1f8f6a', cny: 4.6527 },
  { code: 'CAD', name: '加元', sym: 'C$', flag: '加', color: '#d64545', cny: 5.1402 },
  { code: 'SGD', name: '新加坡元', sym: 'S$', flag: '新', color: '#e05a4f', cny: 5.4816 },
  { code: 'THB', name: '泰铢', sym: '฿', flag: '泰', color: '#3b4fb8', cny: 0.21382 },
  { code: 'CHF', name: '瑞士法郎', sym: 'Fr', flag: '瑞', color: '#cf3a3a', cny: 8.8724 }
]

function fmt(v) {
  if (v === 0) return '0.00'
  if (v >= 1e9) return v.toExponential(4)
  const d = v < 1 ? 4 : 2
  const s = v.toFixed(d)
  const parts = s.split('.')
  parts[0] = parts[0].replace(/\B(?=(\d{3})+(?!\d))/g, ',')
  return parts.join('.')
}

Page({
  data: { names: CUR.map(c => c.name + ' ' + c.code), base: 1, cur: CUR[1], amount: '100', list: [], updated: '2026-10-24 09:30' },
  onLoad() {
    const saved = wx.getStorageSync('tb_rate_base')
    this.setData({ base: typeof saved === 'number' ? saved : 1 })
    this.calc()
  },
  calc() {
    const base = CUR[this.data.base]
    const v = parseFloat(this.data.amount) || 0
    const inCny = v * base.cny
    const list = CUR.filter(c => c.code !== base.code).map(c => ({
      code: c.code, name: c.name, sym: c.sym, flag: c.flag, color: c.color,
      value: fmt(inCny / c.cny),
      unit: '1 ' + base.code + ' = ' + fmt(base.cny / c.cny) + ' ' + c.code
    }))
    this.setData({ cur: base, list })
  },
  onInput(e) { this.setData({ amount: e.detail.value }); this.calc() },
  onPick(e) {
    const i = +e.detail.value
    wx.setStorageSync('tb_rate_base', i)
    this.setData({ base: i }); this.calc()
  },
  onQuick(e) { this.setData({ amount: String(e.currentTarget.dataset.v) }); this.calc() },
  onRow(e) {
    // 点某个币种 = 把它换成基准货币
    const code = e.currentTarget.dataset.code
    const i = CUR.findIndex(c => c.code === code)
    wx.setStorageSync('tb_rate_base', i)
    this.setData({ base: i }); this.calc()
    wx.showToast({ title: '已切换为' + CUR[i].name, icon: 'none' })
  }
})
