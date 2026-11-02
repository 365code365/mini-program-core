const S = require('../../utils/store.js')
const V = require('../../utils/validate.js')
// 添加乘车人；带 id 是编辑（本人的姓名、证件号已实名核验，只能改手机号与票种）
Page({
  data: {
    editId: 0,
    self: false,
    name: '',
    idcard: '',
    phone: '',
    types: ['成人票', '学生票', '儿童票'],
    typeIdx: 0,
    err: '',
    errField: ''
  },
  onLoad(q) {
    const p = q && q.id ? S.findPassenger(Number(q.id)) : null
    if (!p) return
    this.setData({
      editId: p.id, self: !!p.self, name: p.name, idcard: p.idcard, phone: p.phone || '',
      typeIdx: Math.max(0, this.data.types.indexOf(p.type || '成人票'))
    })
    wx.setNavigationBarTitle({ title: '编辑乘车人' })
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
    const editId = this.data.editId
    if (S.passengers().some(p => p.idcard === idcard && p.id !== editId)) return this.fail('idcard', '该乘车人已存在')
    const info = { name: this.data.name.trim(), idcard, phone: this.data.phone.trim(), type: this.data.types[this.data.typeIdx] }
    if (editId) S.updatePassenger(editId, info)
    else S.addPassenger(info)
    this.setData({ err: '', errField: '' })
    wx.showToast({ title: editId ? '已保存' : '已添加', icon: 'success' })
    wx.navigateBack()
  },
  onRemove() {
    wx.showModal({
      title: '删除乘车人',
      content: '确定删除乘车人「' + this.data.name + '」吗？已购车票不受影响',
      confirmText: '删除',
      success: r => {
        if (!r.confirm) return
        S.removePassenger(this.data.editId)
        wx.showToast({ title: '已删除', icon: 'none' })
        wx.navigateBack()
      }
    })
  }
})
