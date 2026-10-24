const TOOLS = [
  { id: 'calc', name: '计算器', desc: '四则运算 / 百分比', ch: '算', c1: '#5c8cff', c2: '#4056ee' },
  { id: 'rate', name: '汇率换算', desc: '12 种常用货币', ch: '汇', c1: '#ffb35c', c2: '#f5733b' },
  { id: 'unit', name: '单位换算', desc: '长度 / 重量 / 温度', ch: '换', c1: '#4fd6a8', c2: '#18a87a' },
  { id: 'bmi', name: 'BMI 计算', desc: '身体质量指数', ch: '体', c1: '#ff8fb1', c2: '#f04f7f' },
  { id: 'loan', name: '房贷计算', desc: '等额本息 / 本金', ch: '贷', c1: '#a68bff', c2: '#7356f0' },
  { id: 'decide', name: '随机决定', desc: '选择困难就抽签', ch: '签', c1: '#5fd0f0', c2: '#2a9fd6' }
]

Page({
  data: { tools: TOOLS, recent: [], greet: '' },
  onShow() {
    const h = new Date().getHours()
    const greet = h < 6 ? '夜深了' : h < 11 ? '早上好' : h < 14 ? '中午好' : h < 18 ? '下午好' : '晚上好'
    const ids = wx.getStorageSync('tb_recent') || []
    const recent = ids.map(id => TOOLS.filter(t => t.id === id)[0]).filter(Boolean)
    this.setData({ recent, greet })
  },
  onTool(e) {
    const id = e.currentTarget.dataset.id
    getApp().touchTool(id)
    wx.navigateTo({ url: '/pages/' + id + '/' + id })
  },
  onFeedback() {
    wx.showModal({ title: '想要新工具？', content: '告诉我们你最常用的计算场景，下个版本优先上线。', confirmText: '好的', showCancel: false })
  }
})
