const { EVENT, GROUPS, ID_TYPES, SIZES, STEPS, DISCLAIMER } = require('../../utils/data.js')
const C = require('../../utils/check.js')
const S = require('../../utils/store.js')

// 生日 picker 没选过时停在哪一天（只是面板初值，不算已填）
const BIRTH_HINT = '2009-06-15'

function emptyForm() {
  return {
    group: -1, name: '', idType: 0, idNo: '', gender: '', birth: '', phone: '', region: [],
    ecName: '', ecPhone: '', size: '', best: '', agree: false
  }
}

Page({
  data: {
    step: 0, steps: STEPS, form: emptyForm(), err: '',
    groups: GROUPS.map(g => ({ name: g.name, dist: g.dist, fee: g.fee, age: g.minAge + '~' + g.maxAge + ' 周岁' })),
    full: [], idTypes: ID_TYPES, sizes: SIZES, disclaimer: DISCLAIMER,
    birthHint: BIRTH_HINT, raceDate: EVENT.raceDate, bestLabel: '未填写', regionLabel: ''
  },
  onLoad(query) {
    const g = getApp().globalData
    const form = g.draft ? Object.assign(emptyForm(), g.draft) : emptyForm()
    if (!g.draft && query.group !== undefined && GROUPS[Number(query.group)]) form.group = Number(query.group)
    this.setData({ form, full: GROUPS.map((x, i) => S.takenOf(i) >= x.quota) })
    this.syncLabels()
  },
  onShow() {
    // 确认页点「修改」回来：跳到对应步骤，填过的都还在
    const g = getApp().globalData
    if (g.jumpStep !== null && g.jumpStep !== undefined) {
      this.setData({ step: g.jumpStep, err: '' })
      g.jumpStep = null
    }
  },
  syncLabels() {
    const f = this.data.form
    this.setData({ bestLabel: C.bestText(f.best), regionLabel: f.region.length ? f.region.join(' ') : '' })
  },
  set(key, v) {
    this.setData({ ['form.' + key]: v, err: '' })
  },
  // ───── 各字段 ─────
  onGroup(e) { this.set('group', Number(e.detail.value)) },
  onName(e) { this.set('name', e.detail.value) },
  onIdType(e) { this.set('idType', Number(e.detail.value)) },
  onIdNo(e) {
    const v = e.detail.value.toUpperCase()
    this.set('idNo', v)
    // 身份证号合法：自动带出生日与性别
    if (this.data.form.idType === 0 && C.idCard(v)) {
      const info = C.idInfo(v)
      this.setData({ 'form.birth': info.birth, 'form.gender': info.gender })
    }
  },
  onGender(e) { this.set('gender', e.detail.value) },
  onBirth(e) { this.set('birth', e.detail.value) },
  onPhone(e) { this.set('phone', e.detail.value) },
  onRegion(e) {
    this.set('region', e.detail.value)
    this.syncLabels()
  },
  onEcName(e) { this.set('ecName', e.detail.value) },
  onEcPhone(e) { this.set('ecPhone', e.detail.value) },
  onSize(e) { this.set('size', e.detail.value) },
  onBest(e) {
    this.set('best', e.detail.value)
    this.syncLabels()
  },
  onAgree(e) { this.set('agree', e.detail.value.indexOf('agree') >= 0) },
  // ───── 每步校验：返回错误文案，空串表示通过 ─────
  check(step) {
    const f = this.data.form
    if (step === 0) {
      if (f.group < 0) return '请选择参赛组别'
      if (this.data.full[f.group]) return GROUPS[f.group].short + '名额已满，请选择其他组别'
      return ''
    }
    if (step === 1) {
      if (!C.name(f.name)) return '请填写真实姓名（2~20 字）'
      if (!f.idNo) return '请填写证件号码'
      if (f.idType === 0 && !C.idCard(f.idNo)) return '身份证号校验不通过'
      if (f.idType !== 0 && !C.otherId(f.idNo)) return '证件号码格式不正确'
      if (!f.gender) return '请选择性别'
      if (!f.birth) return '请选择出生日期'
      if (f.idType === 0 && C.idInfo(f.idNo).birth !== f.birth) return '出生日期与身份证不一致'
      const g = GROUPS[f.group]
      const age = C.age(f.birth, EVENT.raceDate)
      if (age < g.minAge) return g.short + '要求年满 ' + g.minAge + ' 周岁（按比赛日计）'
      if (age > g.maxAge) return g.short + '限 ' + g.maxAge + ' 周岁以下'
      if (!C.phone(f.phone)) return '手机号格式不正确'
      if (f.region.length !== 3) return '请选择所在地区'
      return ''
    }
    if (step === 2) {
      if (!C.name(f.ecName)) return '请填写紧急联系人姓名'
      if (!C.phone(f.ecPhone)) return '紧急联系人手机号格式不正确'
      if (f.ecPhone === f.phone) return '紧急联系人不能是本人'
      if (!f.size) return '请选择服装尺码'
      return ''
    }
    if (step === 3 && !f.agree) return '请阅读并勾选免责声明'
    return ''
  },
  onNext() {
    const msg = this.check(this.data.step)
    if (msg) {
      // 校验失败：停在本步
      this.setData({ err: msg })
      wx.showToast({ title: msg, icon: 'none' })
      return
    }
    if (this.data.step < STEPS.length - 1) {
      this.setData({ step: this.data.step + 1, err: '' })
      wx.pageScrollTo({ scrollTop: 0, duration: 0 })
      return
    }
    getApp().globalData.draft = Object.assign({}, this.data.form)
    wx.navigateTo({ url: '/pages/confirm/confirm' })
  },
  onPrev() {
    if (this.data.step > 0) this.setData({ step: this.data.step - 1, err: '' })
  },
  // 点已完成的步骤直接回去
  onStep(e) {
    const i = Number(e.currentTarget.dataset.i)
    if (i < this.data.step) this.setData({ step: i, err: '' })
  }
})
