const D = require('../../utils/data.js')
// 经停站：从预订页（带 book=1，底部可直接去订票）或订单详情（只看时刻）进来。
// 车次按 from / to / date 重新生成，同一组参数生成的车次与列表页完全一样
Page({
  data: { train: null, dateText: '', list: [], canBook: false, kindText: '' },
  onLoad(q) {
    const from = q.from ? decodeURIComponent(q.from) : '北京'
    const to = q.to ? decodeURIComponent(q.to) : '上海'
    const date = q.date || D.today()
    const all = D.trains(from, to, date)
    const t = all.find(x => x.no === q.no) || all[0]
    const kind = t.type === 'G' ? '高速动车组' : t.type === 'D' ? '动车组' : '普通快速列车'
    wx.setNavigationBarTitle({ title: t.no + ' 经停站' })
    this.train = t
    this.setData({ train: t, dateText: D.dateLabel(t.date), list: D.stops(t), canBook: q.book === '1', kindText: kind })
  },
  onBook() {
    // 预订页就在下面一层：返回即可；直接打开本页时再进预订页
    const pages = getCurrentPages()
    const prev = pages[pages.length - 2]
    if (prev && prev.route === 'pages/book/book') return wx.navigateBack()
    getApp().globalData.train = this.train
    wx.redirectTo({ url: '/pages/book/book?no=' + this.train.no })
  }
})
