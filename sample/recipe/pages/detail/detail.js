const R = require('../../utils/recipes.js')
const S = require('../../utils/store.js')
const MIN_SERVINGS = 1
const MAX_SERVINGS = 10
function mmss(sec) {
  const m = Math.floor(sec / 60)
  const s = sec % 60
  return (m < 10 ? '0' + m : '' + m) + ':' + (s < 10 ? '0' + s : '' + s)
}
Page({
  data: {
    r: null,
    fav: false,
    servings: 2,
    ingredients: [],
    ready: 0,
    step: 0,
    remain: 0,
    remainText: '00:00',
    running: false,
    cookedTimes: 0
  },
  onLoad(q) {
    const r = R.byId(q.id) || R.LIST[0]
    this.r = r
    this.checked = {}
    this.setData({ r, fav: S.isFav(r.id), servings: r.base, cookedTimes: S.cooked()[r.id] || 0 })
    wx.setNavigationBarTitle({ title: r.name })
    this.applyIngredients()
    this.resetTimer()
  },
  onUnload() {
    this.stopTimer()
  },
  // 份量变了：每样食材按 份量 / 基准份量 等比换算
  applyIngredients() {
    const r = this.r
    const ingredients = r.ingredients.map((x, i) => ({
      key: 'i' + i, name: x.name, unit: x.unit, amount: R.scale(x.amount, r.base, this.data.servings), checked: !!this.checked['i' + i]
    }))
    this.setData({ ingredients, ready: ingredients.filter(x => x.checked).length })
  },
  onMinus() {
    if (this.data.servings <= MIN_SERVINGS) return wx.showToast({ title: '至少 1 人份', icon: 'none' })
    this.setData({ servings: this.data.servings - 1 })
    this.applyIngredients()
  },
  onPlus() {
    if (this.data.servings >= MAX_SERVINGS) return wx.showToast({ title: '最多 10 人份', icon: 'none' })
    this.setData({ servings: this.data.servings + 1 })
    this.applyIngredients()
  },
  // checkbox-group：e.detail.value 是勾上的 value 数组
  onCheck(e) {
    this.checked = {}
    e.detail.value.forEach(k => { this.checked[k] = true })
    this.applyIngredients()
    if (this.data.ready === this.data.ingredients.length) wx.showToast({ title: '食材备齐，开做吧', icon: 'none' })
  },
  // swiper 滑动换步骤：计时器换成这一步的时长
  onSwiper(e) {
    if (e.detail.current === this.data.step) return
    this.setData({ step: e.detail.current })
    this.resetTimer()
  },
  onPrevStep() {
    if (this.data.step > 0) this.goStep(this.data.step - 1)
  },
  onNextStep() {
    if (this.data.step < this.r.steps.length - 1) this.goStep(this.data.step + 1)
  },
  goStep(i) {
    this.setData({ step: i })
    this.resetTimer()
  },
  onTimer() {
    if (this.data.running) return this.stopTimer()
    if (this.data.remain <= 0) this.resetTimer()
    this.setData({ running: true })
    this.timer = setInterval(() => {
      const remain = this.data.remain - 1
      this.setData({ remain, remainText: mmss(remain) })
      if (remain <= 0) {
        this.stopTimer()
        wx.showToast({ title: '第 ' + (this.data.step + 1) + ' 步时间到', icon: 'none' })
      }
    }, 1000)
  },
  stopTimer() {
    if (this.timer) clearInterval(this.timer)
    this.timer = null
    if (this.data.running) this.setData({ running: false })
  },
  resetTimer() {
    this.stopTimer()
    const sec = this.r.steps[this.data.step].sec
    this.setData({ remain: sec, remainText: mmss(sec) })
  },
  onFav() {
    const on = S.toggleFav(this.r.id)
    this.setData({ fav: on })
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' })
  },
  onCooked() {
    const n = S.addCooked(this.r.id)
    this.setData({ cookedTimes: n })
    wx.showToast({ title: '打卡成功，第 ' + n + ' 次做这道菜', icon: 'none' })
  }
})
