const A = require('../../utils/account.js')
function pad(n) {
  return n < 10 ? '0' + n : '' + n
}
Page({
  data: { list: [], buyTotal: '0.00', sellTotal: '0.00' },
  onShow() {
    const list = A.orders().map(o => {
      const d = new Date(o.at)
      return Object.assign({}, o, {
        sideText: o.side === 'buy' ? '买入' : '卖出',
        time: pad(d.getHours()) + ':' + pad(d.getMinutes()) + ':' + pad(d.getSeconds()),
        amountText: o.amount.toFixed(2)
      })
    })
    const sum = side => list.filter(o => o.side === side).reduce((s, o) => s + o.amount, 0).toFixed(2)
    this.setData({ list, buyTotal: sum('buy'), sellTotal: sum('sell') })
  }
})
