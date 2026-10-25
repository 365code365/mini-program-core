const D = require('../../utils/devices.js')

Page({
  data: {
    rooms: D.ROOMS,
    current: 0, // 当前房间（tab 与 swiper 共用）
    pages: [], // 每个房间一页设备：[[device]]
    online: 0,
    total: 0,
    running: 0,
    greeting: ''
  },

  onLoad() {
    const h = new Date().getHours()
    this.setData({ greeting: h < 11 ? '早上好' : h < 18 ? '下午好' : '晚上好' })
  },

  // 每次回到首页都从 storage 读（详情页、场景页都会改设备）
  onShow() {
    this.refresh()
  },

  refresh() {
    const list = D.load()
    this.setData({
      pages: D.ROOMS.map(r => list.filter(d => d.room === r.key).map(D.view)),
      online: list.filter(d => d.online).length,
      total: list.length,
      running: list.filter(d => d.online && d.on).length
    })
  },

  onTab(e) {
    this.setData({ current: Number(e.currentTarget.dataset.i) })
  },

  // 手指滑 swiper：同步 tab（只认用户手势，autoplay 之类的来源不改 tab 也无妨）
  onSwiper(e) {
    // 只认 swiper 自己的 change（detail.current 是 number）。本引擎里卡片上 <switch> 的 change
    // 会冒泡到这里（detail 是 { value: true }，微信里 change 不冒泡），不判就把 current 改成 undefined
    if (typeof e.detail.current !== 'number') return
    if (e.detail.current !== this.data.current) this.setData({ current: e.detail.current })
  },

  onSwitch(e) {
    const id = e.currentTarget.dataset.id
    const d = D.update(id, { on: e.detail.value })
    this.refresh()
    wx.showToast({ title: d.name + (d.on ? ' 已打开' : ' 已关闭'), icon: 'none' })
  },

  // 开关外面包一层 catchtap：点开关不触发卡片跳详情
  noop() {},

  onCard(e) {
    const d = D.get(e.currentTarget.dataset.id)
    if (!d.online) {
      wx.showToast({ title: d.name + '离线，请检查网络', icon: 'none' })
      return
    }
    if (d.type === 'ac') wx.navigateTo({ url: '/pages/ac/ac?id=' + d.id })
    else if (d.type === 'light') wx.navigateTo({ url: '/pages/light/light?id=' + d.id })
    else wx.showToast({ title: d.name + '暂无更多设置', icon: 'none' })
  },

  // 一键全关当前房间
  onRoomOff() {
    const room = D.ROOMS[this.data.current]
    const list = D.load()
    let n = 0
    list.forEach(d => {
      if (d.room === room.key && d.online && d.on && d.type !== 'fridge') {
        d.on = false
        n++
      }
    })
    D.save(list)
    this.refresh()
    wx.showToast({ title: n ? room.name + '已关闭 ' + n + ' 台设备' : room.name + '没有开着的设备', icon: 'none' })
  }
})
