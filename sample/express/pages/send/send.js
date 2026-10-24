const D = require('../../utils/data.js')

const CARRIERS = [
  { key: 'sf', name: '顺丰标快', eta: '次日达', first: 18, extra: 6 },
  { key: 'jd', name: '京东特快', eta: '次日达', first: 16, extra: 5 },
  { key: 'zto', name: '中通快递', eta: '1-2 天', first: 8, extra: 3 }
]
const DAYS = ['今天', '明天', '后天']
const SLOTS = ['一小时内', '14:00-16:00', '16:00-18:00', '18:00-20:00']

Page({
  data: {
    sender: null,
    receiver: null,
    goodsList: ['日用品', '数码产品', '衣物', '食品', '文件', '其他'],
    goods: 0,
    weight: 2,
    carrierList: CARRIERS,
    carrier: 0,
    timeRange: [DAYS, SLOTS],
    timeIndex: [0, 0],
    timeText: '今天 一小时内',
    insured: false,
    note: '',
    fee: 0,
    first: 0,
    extra: 0
  },

  onLoad() {
    const list = D.loadAddresses()
    this.setData({
      sender: list.find(a => a.isDefault) || list[0],
      receiver: list[2] || null
    })
    this.calc()
  },

  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 1 })
    const picked = getApp().globalData.pickedAddress
    if (picked) {
      this.setData({ [picked.role]: picked.addr })
      getApp().globalData.pickedAddress = null
    }
  },

  calc() {
    const c = CARRIERS[this.data.carrier]
    const w = Number(this.data.weight)
    const fee = c.first + (w - 1) * c.extra + (this.data.insured ? 3 : 0)
    this.setData({ fee, first: c.first, extra: c.extra })
  },

  onPick(e) {
    wx.navigateTo({ url: '/pages/address/address?select=' + e.currentTarget.dataset.role })
  },

  onGoods(e) {
    this.setData({ goods: Number(e.currentTarget.dataset.i) })
  },

  onWeight(e) {
    this.setData({ weight: e.detail.value })
    this.calc()
  },

  onCarrier(e) {
    this.setData({ carrier: Number(e.currentTarget.dataset.i) })
    this.calc()
  },

  onTime(e) {
    const v = e.detail.value
    this.setData({ timeIndex: v, timeText: DAYS[v[0]] + ' ' + SLOTS[v[1]] })
  },

  onInsure(e) {
    this.setData({ insured: e.detail.value })
    this.calc()
  },

  onNote(e) {
    this.setData({ note: e.detail.value })
  },

  onSubmit() {
    const { sender, receiver } = this.data
    if (!sender || !receiver) {
      wx.showToast({ title: '请先填写寄收件人', icon: 'none' })
      return
    }
    const c = CARRIERS[this.data.carrier]
    wx.showModal({
      title: '确认下单',
      content: c.name + ' · ' + this.data.timeText + ' 上门\n预估运费 ¥' + this.data.fee + '，快递员称重后以实际为准',
      confirmText: '确认',
      confirmColor: '#2f6bff',
      success: res => {
        if (!res.confirm) return
        const no = (c.key === 'sf' ? 'SF' : c.key === 'jd' ? 'JD' : '78') + String(Date.now()).slice(-12)
        const list = D.loadList()
        list.unshift({
          no, carrier: c.key, status: 'pending', remark: '寄件：' + this.data.goodsList[this.data.goods],
          from: sender.region.split(' ')[1].replace('市', ''), to: receiver.region.split(' ')[1].replace('市', ''),
          eta: '等待快递员上门揽收',
          courier: { name: '刘师傅', phone: '137 5810 3326', rate: '4.9', count: '9800' },
          traces: [{ time: '刚刚', text: '订单已提交，快递员将于 ' + this.data.timeText + ' 上门取件' }]
        })
        D.saveList(list)
        wx.showToast({ title: '下单成功', icon: 'success' })
        setTimeout(() => wx.switchTab({ url: '/pages/index/index' }), 800)
      }
    })
  }
})
