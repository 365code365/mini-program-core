const S = require('../../utils/store.js');
const M = require('../../utils/money.js');

Page({
  data: { amtText: '', upper: '', rows: [] },
  onLoad(options) {
    const b = S.bills().find(x => x.id === options.id) || S.bills()[0];
    const acct = S.accounts().find(a => a.id === b.acct);
    this.setData({
      amtText: M.fmt(b.amount),
      upper: M.upper(b.amount),
      rows: [
        { k: '收款人', v: b.counter },
        { k: '收款银行', v: b.bank || '—' },
        { k: '付款账户', v: acct.name + '（' + acct.no.slice(-4) + '）' },
        { k: '账户余额', v: '¥' + M.fmt(acct.balance) },
        { k: '附言', v: b.remark || '无' },
        { k: '转账时间', v: M.timeStr(b.time) },
        { k: '交易流水号', v: 'ZC' + M.dateStr(b.time).replace(/-/g, '') + String(b.time).slice(-6) + b.id.slice(-2) }
      ]
    });
  },
  done() { wx.switchTab({ url: '/pages/index/index' }); },
  again() { wx.redirectTo({ url: '/pages/transfer/transfer' }); },
  bills() { wx.switchTab({ url: '/pages/bills/bills' }); }
});
