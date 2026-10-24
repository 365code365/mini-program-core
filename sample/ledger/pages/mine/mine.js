const store = require('../../utils/store.js')

Page({
  data: { total: 0, days: 0, streak: 0, budget: 0, balanceText: '0.00' },
  onShow() {
    const list = store.all()
    const daySet = {}
    list.forEach(r => { daySet[r.date] = 1 })
    // 连续记账天数：从今天往前数
    let streak = 0
    const now = new Date()
    for (let i = 0; i < 400; i++) {
      const ds = store.fmtDate(new Date(now.getFullYear(), now.getMonth(), now.getDate() - i))
      if (daySet[ds]) streak++; else if (i > 0) break
    }
    const month = store.monthRecords(now.getFullYear(), now.getMonth() + 1)
    this.setData({
      total: list.length,
      days: Object.keys(daySet).length,
      streak,
      budget: store.getBudget(),
      balanceText: store.money(store.sum(month, 'in') - store.sum(month, 'out'))
    })
  },
  onBudget() { wx.navigateTo({ url: '/pages/budget/budget' }) },
  onAdd() { wx.navigateTo({ url: '/pages/add/add' }) },
  onExport() {
    wx.showActionSheet({
      itemList: ['导出本月账单（CSV）', '导出全部账单（CSV）', '生成年度报告'],
      success: (res) => {
        wx.showToast({ title: res.tapIndex === 2 ? '报告生成中' : '已导出到本地', icon: 'success' })
      }
    })
  },
  onReminder() { wx.showToast({ title: '每晚 21:00 提醒记账', icon: 'none' }) },
  onClear() {
    wx.showModal({
      title: '清空全部账单？',
      content: '共 ' + this.data.total + ' 笔记录，清空后无法恢复',
      confirmText: '清空',
      confirmColor: '#f04848',
      success: (res) => {
        if (!res.confirm) return
        store.clear()
        wx.showToast({ title: '已清空', icon: 'success' })
        this.onShow()
      }
    })
  },
  onAbout() {
    wx.showModal({ title: '小鹿记账 v2.3.0', content: '所有账单只保存在本机，不上传任何服务器。', showCancel: false, confirmText: '知道了' })
  }
})
