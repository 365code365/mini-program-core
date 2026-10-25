const M = require('../../utils/market.js')
const A = require('../../utils/account.js')
Page({
  data: {
    list: [],
    sortKey: '',
    sortAsc: false,
    tickNo: 0,
    upCount: 0,
    downCount: 0,
    assets: ''
  },
  onShow() {
    this.refresh()
  },
  onTick() {
    this.refresh()
  },
  refresh() {
    let list = A.watchlist().map(M.quote).filter(Boolean)
    const k = this.data.sortKey
    if (k) {
      const sign = this.data.sortAsc ? 1 : -1
      list = list.slice().sort((a, b) => (a[k] - b[k]) * sign || a.code.localeCompare(b.code))
    }
    this.setData({
      list,
      tickNo: M.ticks(),
      upCount: list.filter(q => q.dir === 'up').length,
      downCount: list.filter(q => q.dir === 'down').length,
      assets: A.summary().total.toFixed(2)
    })
  },
  // 点表头：换一列先降序；同一列再点切换升降序
  onSort(e) {
    const key = e.currentTarget.dataset.key
    if (this.data.sortKey === key) this.setData({ sortAsc: !this.data.sortAsc })
    else this.setData({ sortKey: key, sortAsc: false })
    this.refresh()
  },
  onResetSort() {
    this.setData({ sortKey: '', sortAsc: false })
    this.refresh()
  },
  onOpen(e) {
    wx.navigateTo({ url: '/pages/detail/detail?code=' + e.currentTarget.dataset.code })
  },
  onLong(e) {
    // 微信里 data-code="{{item.code}}" 保留字符串类型；String() 兜住把 '600519' 还原成数字的实现
    const code = String(e.currentTarget.dataset.code)
    const q = M.quote(code)
    wx.showActionSheet({
      itemList: ['删除自选「' + q.name + '」', '去交易'],
      success: res => {
        if (res.tapIndex === 0) {
          A.removeWatch(code)
          wx.showToast({ title: '已删除', icon: 'none' })
          this.refresh()
        } else {
          wx.navigateTo({ url: '/pages/trade/trade?side=buy&code=' + code })
        }
      }
    })
  },
  goSearch() { wx.navigateTo({ url: '/pages/search/search' }) },
  goPositions() { wx.navigateTo({ url: '/pages/positions/positions' }) }
})
