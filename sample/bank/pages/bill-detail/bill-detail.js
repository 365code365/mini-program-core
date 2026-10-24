const S = require('../../utils/store.js');
const M = require('../../utils/money.js');

Page({
  data: { bill: {}, rows: [], note: '', dirty: false, noteTags: ['房租', '家用', '报销', '人情', '投资'] },

  onLoad(options) {
    this.id = options.id;
    const b = S.bills().find(x => x.id === this.id) || S.bills()[0];
    this.id = b.id;
    const acct = S.accounts().find(a => a.id === b.acct) || {};
    this.saved = b.note || '';
    this.setData({
      note: this.saved,
      bill: { title: b.title, type: b.type, g: b.cat.slice(0, 1), amtText: (b.type === 'in' ? '+' : '-') + M.fmt(b.amount) },
      rows: [
        { k: '交易类型', v: b.type === 'in' ? '收入 · ' + b.cat : '支出 · ' + b.cat },
        { k: '交易时间', v: M.timeStr(b.time) },
        { k: '对方信息', v: b.counter },
        { k: '交易账户', v: (acct.name || '') + '（' + String(acct.no || '').slice(-4) + '）' },
        { k: '附言', v: b.remark || '无' },
        { k: '交易流水号', v: 'ZC' + M.dateStr(b.time).replace(/-/g, '') + String(b.time).slice(-6) + b.id.slice(-2) }
      ]
    });
  },

  onNote(e) { this.setData({ note: e.detail.value, dirty: e.detail.value !== this.saved }); },
  useTag(e) {
    const t = e.currentTarget.dataset.t;
    const note = (this.data.note ? this.data.note + ' ' : '') + t;
    this.setData({ note: note.slice(0, 30), dirty: true });
  },
  save() {
    if (!this.data.dirty) return;
    const list = S.bills();
    const b = list.find(x => x.id === this.id);
    b.note = this.data.note;
    S.set('bills', list);
    this.saved = b.note;
    this.setData({ dirty: false });
    wx.showToast({ title: '备注已保存', icon: 'success' });
  },
  report() {
    wx.showActionSheet({
      itemList: ['我没有进行过这笔交易', '金额与实际不符', '联系在线客服'],
      success: r => wx.showToast({ title: r.tapIndex === 2 ? '客服稍后联系您' : '已提交，我们会尽快核实', icon: 'none' })
    });
  }
});
