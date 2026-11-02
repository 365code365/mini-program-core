const store = require('../../utils/store.js')
const host = require('../../utils/host.js')
const RANGES = [{ k: 'month', t: '本月' }, { k: 'last', t: '上月' }, { k: 'all', t: '全部' }]
Page({
  data: { ranges: RANGES, range: 'month', rangeText: '', bookName: '', count: 0, outText: '0.00', inText: '0.00', leftText: '¥0.00', preview: [], more: 0, empty: true, last: '' },
  onLoad() { this.refresh() },
  onRange(e) { this.setData({ range: e.currentTarget.dataset.k }); this.refresh() },
  pick() {
    const now = new Date()
    const r = this.data.range
    if (r === 'all') return { list: store.all().slice().sort(store.byTime), text: '全部' }
    const d = r === 'last' ? new Date(now.getFullYear(), now.getMonth() - 1, 1) : now
    return { list: store.monthRecords(d.getFullYear(), d.getMonth() + 1), text: d.getFullYear() + '年' + (d.getMonth() + 1) + '月' }
  },
  refresh() {
    const p = this.pick()
    this.list = p.list
    const out = store.sum(p.list, 'out')
    const inc = store.sum(p.list, 'in')
    const lines = this.csv().split('\n')
    const last = wx.getStorageSync('ledger_export_last')
    this.setData({
      rangeText: p.text,
      bookName: store.curBook().name,
      count: p.list.length,
      outText: store.money(out),
      inText: store.money(inc),
      leftText: store.yuan(inc - out),
      preview: lines.slice(0, 6),
      more: Math.max(0, lines.length - 6),
      empty: p.list.length === 0,
      last: last && last.at ? last.at + ' 导出了 ' + last.count + ' 笔（' + last.what + '）' : ''
    })
  },
  // CSV：日期,收支,分类,金额,备注（备注里的逗号、引号按 CSV 规则转义）
  csv() {
    const esc = s => (/[",\n]/.test(s) ? '"' + s.replace(/"/g, '""') + '"' : s)
    const rows = ['日期,收支,分类,金额,备注']
    this.list.forEach(r => rows.push([r.date, r.type === 'out' ? '支出' : '收入', r.cat, store.money(r.amount), esc(r.note || '')].join(',')))
    return rows.join('\n')
  },
  text() {
    const d = this.data
    const lines = ['【' + d.bookName + '】' + d.rangeText + '账单', '共 ' + d.count + ' 笔，支出 ¥' + d.outText + '，收入 ¥' + d.inText + '，结余 ' + d.leftText]
    const map = {}
    this.list.forEach(r => { if (r.type === 'out') map[r.cat] = (map[r.cat] || 0) + r.amount })
    Object.keys(map).sort((a, b) => map[b] - map[a]).forEach(k => lines.push(k + ' ¥' + store.money(map[k])))
    return lines.join('\n')
  },
  done(what) {
    const now = new Date()
    wx.setStorageSync('ledger_export_last', { at: (now.getMonth() + 1) + '月' + now.getDate() + '日 ' + store.pad(now.getHours()) + ':' + store.pad(now.getMinutes()), count: this.data.count, what: what })
    this.refresh()
  },
  onCopyCsv() {
    if (this.data.empty) { wx.showToast({ title: '这段时间没有账单', icon: 'none' }); return }
    host.copy(this.csv(), '已复制 ' + this.data.count + ' 行 CSV，可粘贴到表格').then(ok => { if (ok) this.done('CSV') })
  },
  onCopyText() {
    if (this.data.empty) { wx.showToast({ title: '这段时间没有账单', icon: 'none' }); return }
    host.copy(this.text(), '账单摘要已复制').then(ok => { if (ok) this.done('文字摘要') })
  }
})
