const W = require('../../utils/data.js')

const HOT = ['beijing', 'shanghai', 'guangzhou', 'shenzhen', 'chengdu', 'hangzhou', 'xian', 'sanya', 'kunming', 'harbin', 'chongqing', 'xiamen']

Page({
  data: { mine: [], current: '', kw: '', results: [], hot: [], unit: 'C', editing: false },
  onShow() { this.refresh() },
  refresh() {
    const u = W.unit()
    const ids = W.myCities()
    const cur = W.current()
    const now = new Date().getHours()
    const mine = ids.map(id => {
      const c = W.byId(id)
      return { id, name: c.name, prov: c.prov, cond: c.cond, condName: W.COND[c.cond].name, t: W.fmtT(W.tempAt(c, now), u), hi: W.fmtT(c.hi, u), lo: W.fmtT(c.lo, u), bg: W.COND[c.cond].bg, aqi: W.aqiLevel(c.aqi).name }
    })
    const hot = HOT.map(id => { const c = W.byId(id); return { id, name: c.name, added: ids.indexOf(id) >= 0 } })
    this.setData({ mine, current: cur, hot, unit: u })
    this.search(this.data.kw)
  },
  search(kw) {
    kw = (kw || '').trim()
    if (!kw) { this.setData({ results: [] }); return }
    const ids = W.myCities()
    const results = W.CITIES.filter(c => c.name.indexOf(kw) >= 0 || c.prov.indexOf(kw) >= 0 || c.id.indexOf(kw.toLowerCase()) === 0)
      .map(c => ({ id: c.id, name: c.name, prov: c.prov, added: ids.indexOf(c.id) >= 0 }))
    this.setData({ results })
  },
  onInput(e) { this.setData({ kw: e.detail.value }); this.search(e.detail.value) },
  onClearKw() { this.setData({ kw: '', results: [] }) },
  add(id) {
    const ids = W.myCities()
    if (ids.indexOf(id) >= 0) { wx.showToast({ title: '已在列表中', icon: 'none' }); return }
    if (ids.length >= 9) { wx.showToast({ title: '最多添加 9 个城市', icon: 'none' }); return }
    W.setMyCities(ids.concat([id]))
    wx.showToast({ title: '已添加' + W.byId(id).name, icon: 'success' })
    this.setData({ kw: '' })
    this.refresh()
  },
  onAdd(e) { this.add(e.currentTarget.dataset.id) },
  onManage() { this.setData({ editing: !this.data.editing }) },
  onDel(e) {
    const id = e.currentTarget.dataset.id
    const c = W.byId(id)
    wx.showActionSheet({
      alertText: '从我的城市中删除' + c.name + '？',
      itemList: ['删除' + c.name],
      itemColor: '#f04848',
      success: () => {
        const ids = W.myCities()
        if (ids.length <= 1) { wx.showToast({ title: '至少保留一个城市', icon: 'none' }); return }
        W.setMyCities(ids.filter(x => x !== id))
        wx.showToast({ title: '已删除' + c.name, icon: 'none' })
        this.refresh()
      }
    })
  },
  onPick(e) {
    if (this.data.editing) return
    W.setCurrent(e.currentTarget.dataset.id)
    wx.navigateBack()
  },
  onLong(e) {
    const id = e.currentTarget.dataset.id
    const c = W.byId(id)
    wx.showActionSheet({
      itemList: ['设为首页城市', '置顶', '删除'],
      itemColor: '#1f2329',
      success: (res) => {
        let ids = W.myCities()
        if (res.tapIndex === 0) { W.setCurrent(id); wx.showToast({ title: '首页已切换到' + c.name, icon: 'none' }) }
        if (res.tapIndex === 1) { ids = [id].concat(ids.filter(x => x !== id)); W.setMyCities(ids) }
        if (res.tapIndex === 2) {
          if (ids.length <= 1) { wx.showToast({ title: '至少保留一个城市', icon: 'none' }); return }
          W.setMyCities(ids.filter(x => x !== id))
          wx.showToast({ title: '已删除' + c.name, icon: 'none' })
        }
        this.refresh()
      }
    })
  }
})
