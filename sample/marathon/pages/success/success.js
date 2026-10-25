const { EVENT } = require('../../utils/data.js')
const C = require('../../utils/check.js')
const S = require('../../utils/store.js')

Page({
  data: { e: null, event: EVENT, idMasked: '' },
  onLoad(query) {
    this.no = query.no || ''
  },
  onShow() {
    const e = S.findByNo(this.no)
    this.setData({ e, idMasked: e ? C.maskId(e.idNo) : '' })
  },
  onPay() {
    wx.showModal({
      title: '缴纳报名费',
      content: '支付 ¥' + this.data.e.fee + '（演示环境不会真实扣款）',
      confirmText: '支付',
      confirmColor: '#e8491d',
      success: r => {
        if (!r.confirm) return
        const e = S.update(this.no, { status: 'paid', statusText: S.STATUS.paid })
        this.setData({ e })
        wx.showToast({ title: '缴费成功', icon: 'success' })
      }
    })
  },
  goMine() {
    wx.navigateTo({ url: '/pages/mine/mine' })
  },
  goHome() {
    wx.reLaunch({ url: '/pages/index/index' })
  }
})
