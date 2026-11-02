const D = require('../../utils/data.js')
const S = require('../../utils/store.js')

const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']

function buildDays() {
  const out = []
  for (let i = 0; i < 7; i++) {
    const d = new Date()
    d.setDate(d.getDate() + i)
    const md = (d.getMonth() + 1) + '/' + d.getDate()
    out.push({ date: S.dateStr(d.getTime()), md, week: i === 0 ? '今天' : i === 1 ? '明天' : WEEK[d.getDay()], wd: WEEK[d.getDay()] })
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
    canSubmit: false,
    rebook: false
  },

  // ?id=房源；改约时带 rebook=预约 id：沿用原来的联系人，提交后改原记录的时间
  onLoad(q) {
    const saved = S.getContact()
    const old = q.rebook ? S.getBooking(q.rebook) : null
    const src = old || saved
    this.rebook = old ? old.id : 0
    this.setData({
      h: D.get(old ? old.houseId : (q.id || 'h1')),
      days: buildDays(),
      name: src.name || '',
      phone: src.phone || '',
      gender: Number(src.gender) || 0,
      rebook: !!old
    })
    if (old) {
      wx.setNavigationBarTitle({ title: '修改看房时间' })
      const pi = this.data.people.indexOf(old.people)
      this.setData({ peopleIndex: pi >= 0 ? pi : 1, pickup: !!old.pickup, note: old.note || '' })
    }
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
    if (!d.name.trim()) return wx.showToast({ title: '请填写联系人', icon: 'none' })
    if (!/^1\d{10}$/.test(d.phone)) return wx.showToast({ title: '请输入 11 位手机号', icon: 'none' })
    if (!d.agree) return wx.showToast({ title: '请先同意服务协议', icon: 'none' })
    const day = d.days[d.dayIndex]
    const slot = d.slots[d.slot]
    if (d.dayIndex === 0 && Number(slot.text.slice(0, 2)) <= new Date().getHours()) {
      return wx.showToast({ title: '这个时段已经过了，换一个吧', icon: 'none' })
    }
    // 同一套房同一天同一时段只约一次（改约时排除自己）
    const dup = S.getBookings().find(b => b.houseId === d.h.id && b.status === 'pending' && b.day === day.date && b.slot === d.slot && String(b.id) !== String(this.rebook))
    if (dup) return wx.showToast({ title: '这个时段已经约过了', icon: 'none' })
    const time = day.wd + ' ' + day.md + ' ' + slot.text
    wx.showModal({
      title: this.rebook ? '确认改约' : '确认预约',
      content: d.h.community + '\n' + time + '\n' + d.name + (d.gender ? ' 女士' : ' 先生') + ' · ' + d.people[d.peopleIndex] + (d.pickup ? ' · 需接送' : ''),
      confirmText: '确认',
      confirmColor: '#ff5a1f',
      success: res => {
        if (!res.confirm) return
        S.saveContact(d)
        const agent = D.AGENTS[d.h.agent]
        const rec = {
          day: day.date, slot: d.slot, time, name: d.name.trim(), phone: d.phone, gender: d.gender,
          people: d.people[d.peopleIndex], pickup: d.pickup, note: d.note
        }
        if (this.rebook) {
          S.updateBooking(this.rebook, Object.assign(rec, { status: 'pending' }))
          wx.showToast({ title: '已改约', icon: 'success' })
          // 改约是从预约记录进来的：退回去（那一页 onShow 会刷新），别再叠一页预约记录
          return wx.navigateBack()
        } else {
          S.addBooking(Object.assign(rec, {
            houseId: d.h.id, community: d.h.community, cover: d.h.cover, layout: d.h.layout, area: d.h.area,
            agent: agent.name, agentPhone: agent.phone
          }))
          wx.showToast({ title: '预约成功', icon: 'success' })
        }
        wx.redirectTo({ url: '/pages/bookings/bookings' })
      }
    })
  }
})
