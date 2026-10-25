const { EVENT, GROUPS, ID_TYPES } = require('../../utils/data.js')
const C = require('../../utils/check.js')
const S = require('../../utils/store.js')

Page({
  data: { form: null, sum: null, err: '' },
  onShow() {
    // 每次回到确认页都按最新草稿重算（修改后回来）
    const f = getApp().globalData.draft
    if (!f) return this.setData({ form: null })
    const g = GROUPS[f.group]
    this.setData({
      form: f,
      err: '',
      sum: {
        group: g.name + ' · ' + g.dist, fee: g.fee, idType: ID_TYPES[f.idType], idMasked: C.maskId(f.idNo),
        age: C.age(f.birth, EVENT.raceDate), region: f.region.join(' '), best: C.bestText(f.best)
      }
    })
  },
  // 修改：回到向导的对应步骤（向导页还在栈里，填过的都在）
  onEdit(e) {
    const step = Number(e.currentTarget.dataset.step)
    const pages = getCurrentPages()
    const prev = pages[pages.length - 2]
    if (prev && prev.route === 'pages/signup/signup') {
      getApp().globalData.jumpStep = step
      wx.navigateBack()
    } else {
      getApp().globalData.jumpStep = step
      wx.redirectTo({ url: '/pages/signup/signup' })
    }
  },
  onSubmit() {
    const f = this.data.form
    if (S.duplicated(f.idNo)) {
      this.setData({ err: '该证件已报名本届赛事' })
      return wx.showToast({ title: '该证件已报名本届赛事', icon: 'none' })
    }
    const g = GROUPS[f.group]
    if (S.takenOf(f.group) >= g.quota) return wx.showToast({ title: g.short + '名额已满', icon: 'none' })
    wx.showModal({
      title: '提交报名',
      content: '报名' + g.name + '，报名费 ¥' + g.fee + '，提交后请在 24 小时内完成缴费',
      confirmText: '提交',
      confirmColor: '#e8491d',
      success: r => {
        if (!r.confirm) return
        const no = S.nextNo(f.group)
        const list = S.entries()
        list.unshift({
          no, group: f.group, groupName: g.name, fee: g.fee, name: f.name.trim(), idType: f.idType, idNo: f.idNo,
          gender: f.gender, birth: f.birth, phone: f.phone, region: f.region, ecName: f.ecName, ecPhone: f.ecPhone,
          size: f.size, best: f.best, status: 'unpaid', statusText: S.STATUS.unpaid
        })
        S.saveEntries(list)
        getApp().globalData.draft = null
        wx.redirectTo({ url: '/pages/success/success?no=' + no })
      }
    })
  },
  goHome() {
    wx.reLaunch({ url: '/pages/index/index' })
  }
})
