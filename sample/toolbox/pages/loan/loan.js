const YEARS = [5, 10, 15, 20, 25, 30]

function money(v) {
  const s = (Math.round(v * 100) / 100).toFixed(2)
  const p = s.split('.')
  p[0] = p[0].replace(/\B(?=(\d{3})+(?!\d))/g, ',')
  return p.join('.')
}
function wan(v) { return (v / 10000).toFixed(2) }

// 逐月还款计划。method 0 = 等额本息，1 = 等额本金
function schedule(P, years, ratePct, method) {
  const n = years * 12
  const r = ratePct / 100 / 12
  const rows = []
  let left = P
  const fixed = r === 0 ? P / n : P * r * Math.pow(1 + r, n) / (Math.pow(1 + r, n) - 1)
  for (let i = 1; i <= n; i++) {
    const interest = left * r
    let principal, pay
    if (method === 0) { pay = fixed; principal = pay - interest }
    else { principal = P / n; pay = principal + interest }
    if (i === n) { principal = left; pay = principal + interest }
    left = Math.max(0, left - principal)
    rows.push({ pay, principal, interest, left })
  }
  return rows
}

Page({
  data: {
    amount: '200', yearsList: YEARS.map(y => y + ' 年（' + y * 12 + ' 期）'), yi: 5, rate: '3.10', method: 0,
    calced: false, firstPay: '', lastPay: '', totalInterest: '', totalPay: '', loanWan: '', decline: '', years: [], months: [], view: 0
  },
  onLoad() { this.calc() },
  onAmount(e) { this.setData({ amount: e.detail.value }) },
  onRate(e) { this.setData({ rate: e.detail.value }) },
  onYears(e) { this.setData({ yi: +e.detail.value }); this.calc() },
  onMethod(e) { this.setData({ method: +e.currentTarget.dataset.m }); this.calc() },
  onView(e) { this.setData({ view: +e.currentTarget.dataset.v }) },
  onCalc() {
    const a = parseFloat(this.data.amount), r = parseFloat(this.data.rate)
    if (!(a > 0) || a > 10000) { wx.showToast({ title: '贷款金额 0~10000 万', icon: 'none' }); return }
    if (!(r >= 0) || r > 20) { wx.showToast({ title: '年利率 0~20%', icon: 'none' }); return }
    this.calc()
    wx.showToast({ title: '已重新计算', icon: 'success' })
  },
  calc() {
    const P = (parseFloat(this.data.amount) || 0) * 10000
    const yrs = YEARS[this.data.yi]
    const rows = schedule(P, yrs, parseFloat(this.data.rate) || 0, this.data.method)
    let ti = 0, tp = 0
    rows.forEach(x => { ti += x.interest; tp += x.pay })
    const years = []
    for (let y = 0; y < yrs; y++) {
      let pay = 0, pr = 0, it = 0
      for (let m = 0; m < 12; m++) { const x = rows[y * 12 + m]; pay += x.pay; pr += x.principal; it += x.interest }
      years.push({ y: '第' + (y + 1) + '年', pay: wan(pay), pr: wan(pr), it: wan(it), left: wan(rows[y * 12 + 11].left) })
    }
    const months = rows.slice(0, 12).map((x, i) => ({ m: i + 1, pay: money(x.pay), pr: money(x.principal), it: money(x.interest) }))
    this.setData({
      calced: true,
      firstPay: money(rows[0].pay),
      lastPay: money(rows[rows.length - 1].pay),
      decline: rows.length > 1 ? money(rows[0].pay - rows[1].pay) : '0.00',
      totalInterest: wan(ti), totalPay: wan(tp), loanWan: wan(P), years, months
    })
  }
})
