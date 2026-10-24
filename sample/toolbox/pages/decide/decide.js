const TOPICS = [
  { name: '今天吃什么', items: ['黄焖鸡米饭', '兰州拉面', '麻辣香锅', '螺蛳粉', '沙县小吃', '日式拉面', '轻食沙拉', '肯德基'] },
  { name: '周末去哪儿', items: ['城市公园', '美术馆', '看电影', '爬山', '逛商场', '露营', '宅家追剧', '探店咖啡'] },
  { name: '喝什么奶茶', items: ['喜茶 多肉葡萄', '霸王茶姬 伯牙绝弦', '蜜雪冰城 柠檬水', '茶百道 杨枝甘露', '古茗 布蕾脆脆奶芙', '瑞幸 生椰拿铁', '奈雪 霸气橙子', '一点点 波霸奶茶'] },
  { name: '谁去拿外卖', items: ['小王', '小李', '老张', '阿杰', 'Lily', '大刘', '你自己', '石头剪刀布'] }
]
// 3×3 格子里外圈 8 格的顺时针顺序（中间第 4 格是按钮）
const RING = [0, 1, 2, 5, 8, 7, 6, 3]

Page({
  data: { topics: TOPICS, ti: 0, title: TOPICS[0].name, cells: [], active: -1, running: false, result: '', history: [], custom: [], draft: '' },
  onLoad() {
    const custom = wx.getStorageSync('tb_decide_custom') || []
    this.setData({ custom, history: wx.getStorageSync('tb_decide_history') || [] })
    this.build(TOPICS[0].items)
  },
  onUnload() { clearTimeout(this.timer) },
  build(items) {
    const cells = []
    for (let i = 0; i < 9; i++) cells.push({ i, text: '', ring: RING.indexOf(i) })
    RING.forEach((pos, k) => { cells[pos].text = items[k % items.length] || '' })
    this.items = items
    this.setData({ cells, active: -1, result: '' })
  },
  onTopic() {
    if (this.data.running) return
    const names = TOPICS.map(t => t.name)
    names.push('自定义选项（' + this.data.custom.length + '）')
    // showActionSheet 最多 6 项
    wx.showActionSheet({
      itemList: names,
      success: (res) => {
        const i = res.tapIndex
        if (i < TOPICS.length) { this.setData({ ti: i, title: TOPICS[i].name }); this.build(TOPICS[i].items) }
        else this.useCustom()
      }
    })
  },
  useCustom() {
    if (this.data.custom.length < 2) { wx.showToast({ title: '先在下方添加至少 2 个选项', icon: 'none' }); return }
    this.setData({ ti: -1, title: '我的选项' }); this.build(this.data.custom)
  },
  onDraft(e) { this.setData({ draft: e.detail.value }) },
  onAddOpt() {
    const t = this.data.draft.trim()
    if (!t) { wx.showToast({ title: '请输入选项', icon: 'none' }); return }
    if (this.data.custom.length >= 8) { wx.showToast({ title: '最多 8 个选项', icon: 'none' }); return }
    const custom = this.data.custom.concat([t])
    wx.setStorageSync('tb_decide_custom', custom)
    this.setData({ custom, draft: '' })
    if (this.data.ti === -1) this.build(custom)
  },
  onDelOpt(e) {
    const i = e.currentTarget.dataset.i
    const custom = this.data.custom.filter((x, k) => k !== i)
    wx.setStorageSync('tb_decide_custom', custom)
    this.setData({ custom })
    if (this.data.ti === -1 && custom.length >= 2) this.build(custom)
  },
  onStart() {
    if (this.data.running) return
    const target = Math.floor(Math.random() * 8)
    const total = 8 * 3 + target   // 先转三圈
    let step = 0
    this.setData({ running: true, result: '' })
    const tick = () => {
      this.setData({ active: RING[step % 8] })
      if (step >= total) return this.finish(RING[step % 8])
      step++
      // 越到后面越慢，像真的转盘在减速
      const left = total - step
      const delay = left > 10 ? 50 : 50 + (10 - left) * 28
      this.timer = setTimeout(tick, delay)
    }
    tick()
  },
  finish(pos) {
    const text = this.data.cells[pos].text
    const history = [{ t: this.data.title, r: text }].concat(this.data.history).slice(0, 5)
    wx.setStorageSync('tb_decide_history', history)
    this.setData({ running: false, result: text, history })
    wx.showModal({ title: '就决定是你了', content: text, confirmText: '就它了', cancelText: '再抽一次', confirmColor: '#4056ee',
      success: (res) => { if (res.cancel) this.onStart() } })
  }
})
