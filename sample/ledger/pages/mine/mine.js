const store = require('../../utils/store.js')

Page({
  data: { total: 0, days: 0, streak: 0, budget: 0, balanceText: '0.00', bookName: '', bookTotal: 1, catTotal: 0, remindText: '' },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 })
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
      bookName: store.curBook().name,
      bookTotal: store.books().length,
      catTotal: store.cats('out').length + store.cats('in').length,
      remindText: this.remindText(store.reminder()),
      balanceText: store.money(store.sum(month, 'in') - store.sum(month, 'out'))
    })
  },
  remindText(r) {
    if (!r.on) return '已关闭'
    return (r.days === 'workday' ? '工作日 ' : '每天 ') + r.time
  },
  onBudget() { wx.navigateTo({ url: '/pages/budget/budget' }) },
  onAdd() { wx.navigateTo({ url: '/pages/add/add' }) },
  onBooks() { wx.navigateTo({ url: '/pages/books/books' }) },
  onCats() { wx.navigateTo({ url: '/pages/cats/cats' }) },
  onExport() { wx.navigateTo({ url: '/pages/export/export' }) },
  onReminder() { wx.navigateTo({ url: '/pages/remind/remind' }) },
  onList() { wx.switchTab({ url: '/pages/index/index' }) },
  onStats() { wx.switchTab({ url: '/pages/stats/stats' }) },
  onClear() {
    wx.showModal({
      title: '清空本账本账单？',
      content: '「' + this.data.bookName + '」共 ' + this.data.total + ' 笔记录，清空后无法恢复',
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
