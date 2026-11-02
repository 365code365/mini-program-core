// 地址簿：增 / 删 / 改 / 设默认 / 导入微信地址；从寄件页带 select=sender|receiver 进来时点地址即选中
const S = require('../../utils/store.js')
const host = require('../../utils/host.js')
const EMPTY = { id: 0, name: '', phone: '', region: '', regionArr: [], detail: '', tag: '家', isDefault: false }
Page({
  data: {
    list: [],
    selectRole: '',
    showForm: false,
    form: EMPTY,
    formSeq: 0,
    tags: ['家', '公司', '父母', '朋友'],
    tagColors: { '家': '#ff6a1a', '公司': '#2f6bff', '父母': '#19a974', '朋友': '#7c4dff' }
  },
  onLoad(query) {
    this.setData({ selectRole: query.select === 'sender' || query.select === 'receiver' ? query.select : '' })
    if (this.data.selectRole) wx.setNavigationBarTitle({ title: this.data.selectRole === 'sender' ? '选择寄件人' : '选择收件人' })
    this.refresh()
  },
  refresh() {
    this.setData({ list: S.getAddrs().map(a => Object.assign({}, a, { phoneText: S.maskPhone(a.phone) })) })
  },
  onSelect(e) {
    if (!this.data.selectRole) return
    const addr = this.data.list[Number(e.currentTarget.dataset.i)]
    getApp().globalData.pickedAddress = { role: this.data.selectRole, addr }
    wx.navigateBack()
  },
  onDefault(e) {
    S.setDefaultAddr(this.data.list[Number(e.currentTarget.dataset.i)].id)
    this.refresh()
    wx.showToast({ title: '已设为默认', icon: 'none' })
  },
  onMore(e) {
    const a = this.data.list[Number(e.currentTarget.dataset.i)]
    wx.showActionSheet({
      itemList: ['编辑', '复制地址', '删除'],
      success: res => {
        if (res.tapIndex === 0) this.edit(a)
        else if (res.tapIndex === 1) host.copy(a.name + ' ' + a.phone + ' ' + a.region + ' ' + a.detail, '地址已复制')
        else {
          wx.showModal({
            title: '删除地址',
            content: '确定删除「' + a.name + ' · ' + a.tag + '」吗？',
            confirmColor: '#f2453d',
            success: r => {
              if (!r.confirm) return
              S.removeAddr(a.id)
              this.refresh()
              wx.showToast({ title: '已删除', icon: 'none' })
            }
          })
        }
      }
    })
  },
  edit(a) {
    this.setData({
      showForm: true, formSeq: this.data.formSeq + 1,
      form: { id: a.id, name: a.name, phone: a.phone, region: a.region, regionArr: a.region.split(' '), detail: a.detail, tag: a.tag, isDefault: !!a.isDefault }
    })
    wx.pageScrollTo({ scrollTop: 99999, duration: 0 })
  },
  onAdd() {
    this.setData({ showForm: true, formSeq: this.data.formSeq + 1, form: Object.assign({}, EMPTY, { regionArr: [] }) })
    wx.pageScrollTo({ scrollTop: 99999, duration: 0 })
  },
  onImport() {
    host.chooseAddress().then(r => {
      if (!r) return
      this.setData({
        formSeq: this.data.formSeq + 1,
        form: Object.assign({}, this.data.form, { name: r.name, phone: r.phone, regionArr: r.regionArr, region: r.regionArr.join(' '), detail: r.detail })
      })
    })
  },
  onCancel() {
    this.setData({ showForm: false })
  },
  onField(e) {
    this.setData({ ['form.' + e.currentTarget.dataset.k]: e.detail.value })
  },
  onRegion(e) {
    this.setData({ 'form.regionArr': e.detail.value, 'form.region': e.detail.value.join(' ') })
  },
  onTag(e) {
    this.setData({ 'form.tag': String(e.currentTarget.dataset.t) })
  },
  onSave() {
    const f = this.data.form
    const name = String(f.name).trim()
    const phone = String(f.phone).replace(/\s/g, '')
    const detail = String(f.detail).trim()
    let err = ''
    if (!name) err = '请填写联系人'
    else if (!/^1\d{10}$/.test(phone)) err = '请填写 11 位手机号'
    else if (!f.region) err = '请选择所在地区'
    else if (detail.length < 4) err = '请填写详细地址（街道、楼牌号）'
    if (err) {
      wx.showToast({ title: err, icon: 'none' })
      return
    }
    S.saveAddr({ id: f.id, name, phone, region: f.region, detail, tag: f.tag, isDefault: f.isDefault })
    this.setData({ showForm: false })
    this.refresh()
    wx.showToast({ title: f.id ? '已保存修改' : '已添加', icon: 'success' })
  }
})
