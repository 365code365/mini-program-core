const D = require('../../utils/data.js')

const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']

function buildDays() {
  const out = []
  for (let i = 0; i < 7; i++) {
    const d = new Date()
    d.setDate(d.getDate() + i)
    const md = (d.getMonth() + 1) + '/' + d.getDate()
    out.push({ date: d.getFullYear() + '-' + md, md, week: i === 0 ? '今天' : i === 1 ? '明天' : WEEK[d.getDay()] })
  }
  return out
}

Page({
  data: {
    h: null,
    days: [],
    dayIndex: 1,
    slots: [
      { text: '09:00-11:00', full: false },
      { text: '11:00-13:00', full: true },
      { text: '14:00-16:00', full: false },
      { text: '16:00-18:00', full: false },
      { text: '18:00-20:00', full: false },
      { text: '20:00-21:00', full: true }
    ],
    slot: 2,
    name: '',
    gender: 0,
    phone: '',
    people: ['1 人', '2 人', '3 人', '4 人及以上'],
    peopleIndex: 1,
    pickup: false,
    note: '',
    agree: true,
    canSubmit: false
  },

  onLoad(q) {
    const saved = wx.getStorageSync('house_contact') || {}
    this.setData({
      h: D.get(q.id || 'h1'),
      days: buildDays(),
      name: saved.name || '',
      phone: saved.phone || '',
      gender: saved.gender || 0
    })
    this.check()
  },

  check() {
    const { name, phone, agree, slot } = this.data
    this.setData({ canSubmit: !!name && /^1\d{10}$/.test(phone) && agree && slot >= 0 })
  },

  onDay(e) { this.setData({ dayIndex: Number(e.currentTarget.dataset.i) }) },

  onSlot(e) {
    const i = Number(e.currentTarget.dataset.i)
    if (this.data.slots[i].full) {
      wx.showToast({ title: '该时段已约满', icon: 'none' })
      return
    }
    this.setData({ slot: i })
    this.check()
  },

  onName(e) { this.setData({ name: e.detail.value }); this.check() },
  onPhone(e) { this.setData({ phone: e.detail.value }); this.check() },
  onGender(e) { this.setData({ gender: Number(e.currentTarget.dataset.g) }) },
  onPeople(e) { this.setData({ peopleIndex: Number(e.detail.value) }) },
  onPickup(e) { this.setData({ pickup: e.detail.value }) },
  onNote(e) { this.setData({ note: e.detail.value }) },
  onAgree() { this.setData({ agree: !this.data.agree }); this.check() },

  onSubmit() {
    const d = this.data
    if (!d.name) return wx.showToast({ title: '请填写联系人', icon: 'none' })
    if (!/^1\d{10}$/.test(d.phone)) return wx.showToast({ title: '请输入 11 位手机号', icon: 'none' })
    if (!d.agree) return wx.showToast({ title: '请先同意服务协议', icon: 'none' })
    const day = d.days[d.dayIndex]
    const time = day.week + ' ' + day.md + ' ' + d.slots[d.slot].text
    wx.showModal({
      title: '确认预约',
      content: d.h.community + '\n' + time + '\n' + d.name + (d.gender ? ' 女士' : ' 先生') + ' · ' + d.people[d.peopleIndex] + (d.pickup ? ' · 需接送' : ''),
      confirmText: '确认',
      confirmColor: '#ff5a1f',
      success: res => {
        if (!res.confirm) return
        wx.setStorageSync('house_contact', { name: d.name, phone: d.phone, gender: d.gender })
        D.addBooking({
          id: Date.now(), houseId: d.h.id, community: d.h.community, cover: d.h.cover,
          layout: d.h.layout, area: d.h.area, time, agent: D.AGENTS[d.h.agent].name, status: '待带看'
        })
        wx.showToast({ title: '预约成功', icon: 'success' })
        setTimeout(() => wx.navigateBack(), 900)
      }
    })
  }
})
