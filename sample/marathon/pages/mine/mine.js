const C = require('../../utils/check.js')
const S = require('../../utils/store.js')

Page({
  data: { list: [], activeCount: 0 },
  onShow() {
    this.refresh()
  },
  refresh() {
    const list = S.entries().map(e => ({ no: e.no, name: e.name, groupName: e.groupName, fee: e.fee, status: e.status, statusText: e.statusText, idMasked: C.maskId(e.idNo) }))
    this.setData({ list, activeCount: list.filter(e => e.status !== 'canceled').length })
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/success/success?no=' + e.currentTarget.dataset.no })
  },
  onCancel(e) {
    const no = e.currentTarget.dataset.no
    const it = this.data.list.find(i => i.no === no)
    wx.showModal({
      title: '取消报名',
      content: '取消「' + it.groupName + '」报名后名额将释放' + (it.status === 'paid' ? '，报名费 ¥' + it.fee + ' 原路退回' : '') + '，确定取消？',
      confirmText: '取消报名',
      cancelText: '再想想',
      confirmColor: '#f2453d',
      success: r => {
        if (!r.confirm) return
        S.update(no, { status: 'canceled', statusText: S.STATUS.canceled })
        this.refresh()
        wx.showToast({ title: '已取消', icon: 'none' })
      }
    })
  },
  goHome() {
    wx.reLaunch({ url: '/pages/index/index' })
  }
})
