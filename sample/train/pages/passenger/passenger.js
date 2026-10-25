const S = require('../../utils/store.js')
const V = require('../../utils/validate.js')
Page({
  data: {
    name: '',
    idcard: '',
    phone: '',
    types: ['成人票', '学生票', '儿童票'],
    typeIdx: 0,
    err: '',
    errField: ''
  },
  onName(e) {
    this.setData({ name: e.detail.value, errField: this.data.errField === 'name' ? '' : this.data.errField })
  },
  onId(e) {
    this.setData({ idcard: e.detail.value, errField: this.data.errField === 'idcard' ? '' : this.data.errField })
  },
  onPhone(e) {
    this.setData({ phone: e.detail.value, errField: this.data.errField === 'phone' ? '' : this.data.errField })
  },
  onType(e) {
    this.setData({ typeIdx: Number(e.detail.value) })
  },
  fail(field, msg) {
    this.setData({ err: msg, errField: field })
    wx.showToast({ title: msg, icon: 'none' })
  },
  // 依次校验：姓名 → 身份证 → 手机号，第一个不合格的弹 toast 并标红
  onSave() {
    const checks = [['name', V.name(this.data.name)], ['idcard', V.idcard(this.data.idcard)], ['phone', V.phone(this.data.phone)]]
    const bad = checks.find(c => c[1])
    if (bad) return this.fail(bad[0], bad[1])
    const idcard = this.data.idcard.trim().toUpperCase()
    if (S.passengers().some(p => p.idcard === idcard)) return this.fail('idcard', '该乘车人已存在')
    S.addPassenger({ name: this.data.name.trim(), idcard, phone: this.data.phone.trim(), type: this.data.types[this.data.typeIdx] })
    this.setData({ err: '', errField: '' })
    wx.showToast({ title: '已添加', icon: 'success' })
    wx.navigateBack()
  }
})
