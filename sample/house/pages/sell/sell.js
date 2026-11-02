// 卖房估价 / 委托出售出租：填房屋信息 → 估价 → 填联系方式提交委托 → 「我的委托」里看进度、撤销、联系经纪人
const D = require('../../utils/data.js')
const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
const YEARS = []
for (let y = 2025; y >= 1995; y--) YEARS.push(y + ' 年')
// 委托按区分给门店经纪人
const AGENT_OF = { 滨江: 'a1', 上城: 'a1', 西湖: 'a2', 拱墅: 'a2', 余杭: 'a3' }
Page({
  data: {
    modes: ['我要卖房', '我要出租'],
    mode: 0,
    community: '',
    districts: D.DISTRICTS.map(d => d.name),
    dIndex: 0,
    area: '',
    rooms: ['一室', '两室', '三室', '四室', '五室及以上'],
    rIndex: 1,
    floors: D.FLOORS,
    fIndex: 1,
    decos: D.DECOS,
    deIndex: 2,
    years: YEARS,
    yIndex: 10,
    est: null,
    expect: '',
    name: '',
    phone: '',
    formSeq: 0,
    entrusts: []
  },
  onLoad(q) {
    const c = S.getContact()
    this.setData({ mode: q.mode === 'rent' ? 1 : 0, name: c.name || '', phone: c.phone || '' })
  },
  onShow() {
    this.setData({ entrusts: S.getEntrusts() })
  },
  onMode(e) {
    // 售价（万）与月租（元）不是一个量纲：换模式清掉期望价格
    this.setData({ mode: Number(e.currentTarget.dataset.i), est: null, expect: '' })
  },
  onInput(e) {
    this.setData({ [e.currentTarget.dataset.k]: e.detail.value })
    if (e.currentTarget.dataset.k === 'area') this.setData({ est: null })
  },
  onPick(e) {
    this.setData({ [e.currentTarget.dataset.k]: Number(e.detail.value), est: null })
  },
  validHouse() {
    const d = this.data
    if (!d.community.trim()) return wx.showToast({ title: '请填写小区名称', icon: 'none' }), false
    const area = parseFloat(d.area)
    if (!(area >= 10 && area <= 1000)) return wx.showToast({ title: '请填写 10~1000 ㎡ 的面积', icon: 'none' }), false
    return true
  },
  onEstimate() {
    if (!this.validHouse()) return
    const d = this.data
    const r = D.estimate({ district: d.districts[d.dIndex], area: parseFloat(d.area), floor: d.fIndex, deco: d.deIndex, year: 2025 - d.yIndex })
    // 出租：按售价折算月租（杭州租售比约 1:600）
    const est = d.mode === 0
      ? { main: r.low + '~' + r.high, unit: '万', sub: '参考单价 ' + r.unit + ' 元/㎡', value: r.total }
      : (() => {
        const m = Math.round(r.total * 10000 / 600 / 100) * 100
        return { main: Math.round(m * 0.95) + '~' + Math.round(m * 1.05), unit: '元/月', sub: '同小区同户型近 3 个月成交参考', value: m }
      })()
    this.setData({ est, expect: this.data.expect || String(est.value) })
  },
  onSubmit() {
    if (!this.validHouse()) return
    const d = this.data
    if (!d.est) return wx.showToast({ title: '先点「免费估价」看看行情', icon: 'none' })
    const expect = parseFloat(d.expect)
    if (!(expect > 0)) return wx.showToast({ title: '请填写期望价格', icon: 'none' })
    if (!d.name.trim()) return wx.showToast({ title: '请填写联系人', icon: 'none' })
    if (!/^1\d{10}$/.test(d.phone)) return wx.showToast({ title: '请输入 11 位手机号', icon: 'none' })
    const district = d.districts[d.dIndex]
    const agent = D.AGENTS[AGENT_OF[district]]
    const unit = d.mode === 0 ? '万' : '元/月'
    wx.showModal({
      title: d.mode === 0 ? '提交卖房委托' : '提交出租委托',
      content: d.community.trim() + ' · ' + d.area + '㎡ · ' + d.rooms[d.rIndex] + '\n期望 ' + expect + ' ' + unit + '\n提交后由' + agent.store + ' ' + agent.name + '联系你上门核验',
      confirmText: '提交',
      confirmColor: '#ff5a1f',
      success: r => {
        if (!r.confirm) return
        S.saveContact({ name: d.name.trim(), phone: d.phone, gender: S.getContact().gender || 0 })
        S.addEntrust({
          mode: d.mode === 0 ? 'sell' : 'rent', community: d.community.trim(), district, area: parseFloat(d.area),
          rooms: d.rooms[d.rIndex], floor: d.floors[d.fIndex], deco: d.decos[d.deIndex], year: d.years[d.yIndex],
          expect, unit, estimate: d.est.main + d.est.unit, name: d.name.trim(), phone: d.phone,
          agent: agent.name, agentPhone: agent.phone
        })
        wx.showToast({ title: '委托已提交', icon: 'success' })
        this.setData({ community: '', area: '', expect: '', est: null, formSeq: d.formSeq + 1, entrusts: S.getEntrusts() })
      }
    })
  },
  onWithdraw(e) {
    const id = e.currentTarget.dataset.id
    wx.showModal({
      title: '撤销委托',
      content: '撤销后房源不再对外展示，经纪人也不会再联系你。',
      confirmText: '撤销',
      confirmColor: '#f2453d',
      success: r => {
        if (!r.confirm) return
        S.withdrawEntrust(id)
        this.setData({ entrusts: S.getEntrusts() })
        wx.showToast({ title: '已撤销', icon: 'none' })
      }
    })
  },
  onCallAgent(e) {
    const it = this.data.entrusts.find(x => String(x.id) === String(e.currentTarget.dataset.id))
    host.call(it.agentPhone, '经纪人 ' + it.agent)
  }
})
