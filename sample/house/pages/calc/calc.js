// 房贷计算：商贷 / 公积金，等额本息 / 等额本金。利率按 2026 年 LPR（5 年以上 3.5%）与公积金 2.85%
const RATES = [3.5, 2.85]
const RATIOS = [20, 30, 40, 50, 60, 70]
const YEARS = [5, 10, 15, 20, 25, 30]
function fmt(n) {
  return String(Math.round(n)).replace(/\B(?=(\d{3})+(?!\d))/g, ',')
}
Page({
  data: {
    kinds: ['商业贷款', '公积金贷款'],
    kind: 0,
    methods: ['等额本息', '等额本金'],
    method: 0,
    price: '300',
    ratioText: RATIOS.map(r => r + '%'),
    ratioIndex: 1,
    yearText: YEARS.map(y => y + ' 年'),
    yearIndex: 5,
    rate: RATES[0],
    res: null,
    formSeq: 0
  },
  onLoad(q) {
    if (Number(q.price) > 0) this.setData({ price: String(q.price) })
    this.calc()
  },
  onKind(e) {
    const kind = Number(e.currentTarget.dataset.i)
    this.setData({ kind, rate: RATES[kind] })
    this.calc()
  },
  onMethod(e) {
    this.setData({ method: Number(e.currentTarget.dataset.i) })
    this.calc()
  },
  onPrice(e) {
    this.setData({ price: e.detail.value })
    this.calc()
  },
  onRatio(e) {
    this.setData({ ratioIndex: Number(e.detail.value) })
    this.calc()
  },
  onYear(e) {
    this.setData({ yearIndex: Number(e.detail.value) })
    this.calc()
  },
  calc() {
    const price = parseFloat(this.data.price)
    if (!(price > 0) || price > 100000) {
      this.setData({ res: null })
      return
    }
    // 公积金贷款杭州单笔上限 120 万（多出来的部分提示转商贷）
    let loan = price * 10000 * (1 - RATIOS[this.data.ratioIndex] / 100)
    let capped = false
    if (this.data.kind === 1 && loan > 1200000) {
      loan = 1200000
      capped = true
    }
    const n = YEARS[this.data.yearIndex] * 12
    const r = this.data.rate / 100 / 12
    let first, last, interest
    if (this.data.method === 0) {
      first = loan * r * Math.pow(1 + r, n) / (Math.pow(1 + r, n) - 1)
      last = first
      interest = first * n - loan
    } else {
      const base = loan / n
      first = base + loan * r
      last = base + base * r
      interest = (n + 1) * loan * r / 2
    }
    this.setData({
      res: {
        down: fmt(price * 10000 - loan), loan: fmt(loan), first: fmt(first), last: fmt(last),
        interest: fmt(interest), total: fmt(loan + interest), months: n, capped, equal: this.data.method === 0
      }
    })
  },
  onReset() {
    this.setData({ kind: 0, method: 0, price: '300', ratioIndex: 1, yearIndex: 5, rate: RATES[0], formSeq: this.data.formSeq + 1 })
    this.calc()
  },
  // 按总价落到二手房的价格档（400 万以下 / 400-500 万 / 500 万以上）
  onFind() {
    const price = parseFloat(this.data.price)
    if (!(price > 0)) return wx.showToast({ title: '请先输入房屋总价', icon: 'none' })
    const p = price < 400 ? 1 : price < 500 ? 2 : 3
    wx.navigateTo({ url: '/pages/list/list?type=second&p=' + p })
  }
})
