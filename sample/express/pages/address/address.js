const D = require('../../utils/data.js')

const EMPTY = { name: '', phone: '', region: '', regionArr: [], detail: '', tag: '家' }

Page({
  data: {
    list: [],
    selectRole: '',
    showForm: false,
    form: EMPTY,
    tags: ['家', '公司', '父母', '朋友'],
    tagColors: { '家': '#ff6a1a', '公司': '#2f6bff', '父母': '#19a974', '朋友': '#7c4dff' }
  },

  onLoad(query) {
    this.setData({ selectRole: query.select || '' })
    if (query.select) wx.setNavigationBarTitle({ title: query.select === 'sender' ? '选择寄件人' : '选择收件人' })
    this.refresh()
  },

  refresh() {
    this.setData({ list: D.loadAddresses() })
  },

  save(list) {
    wx.setStorageSync('express_addr', list)
    this.setData({ list })
  },

  onSelect(e) {
    if (!this.data.selectRole) return
    const addr = this.data.list[Number(e.currentTarget.dataset.i)]
    getApp().globalData.pickedAddress = { role: this.data.selectRole, addr }
    wx.navigateBack()
  },

  onDefault(e) {
    const i = Number(e.currentTarget.dataset.i)
    const list = this.data.list.map((a, k) => Object.assign({}, a, { isDefault: k === i }))
    this.save(list)
    wx.showToast({ title: '已设为默认', icon: 'none' })
  },

  onMore(e) {
    const i = Number(e.currentTarget.dataset.i)
    wx.showActionSheet({
      itemList: ['复制地址', '删除'],
      itemColor: '#1f2329',
      success: res => {
        const a = this.data.list[i]
        if (res.tapIndex === 0) {
          wx.setClipboardData({ data: a.name + ' ' + a.phone + ' ' + a.region + ' ' + a.detail })
        } else {
          wx.showModal({
            title: '删除地址',
            content: '确定删除「' + a.name + ' · ' + a.tag + '」吗？',
            confirmColor: '#f2453d',
            success: r => {
              if (!r.confirm) return
              const list = this.data.list.slice()
              list.splice(i, 1)
              this.save(list)
            }
          })
        }
      }
    })
  },

  onAdd() {
    this.setData({ showForm: true, form: Object.assign({}, EMPTY) })
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
    this.setData({ 'form.tag': e.currentTarget.dataset.t })
  },

  onSave() {
    const f = this.data.form
    if (!f.name || !/^1\d{10}$/.test(f.phone) || !f.region || !f.detail) {
      wx.showToast({ title: '请完整填写地址信息', icon: 'none' })
      return
    }
    const list = this.data.list.slice()
    list.push({
      id: Date.now(), name: f.name, phone: f.phone.slice(0, 3) + '****' + f.phone.slice(7),
      region: f.region, detail: f.detail, tag: f.tag, isDefault: false
    })
    this.save(list)
    this.setData({ showForm: false })
    wx.showToast({ title: '已保存', icon: 'success' })
  }
})
