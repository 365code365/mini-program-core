const S = require('../../utils/store.js');
const host = require('../../utils/host.js');
const { RECHARGES } = require('../../utils/data.js');
const TYPE = { recharge: '充值', pay: '消费', refund: '退款', carry: '结转' };

Page({
  data: { balanceText: '0', options: [], pick: 1, need: 0, needText: '', logs: [] },
  onLoad(q) {
    // 从支付页「余额不足」过来：带着还差多少（分），默认选第一档够付的
    const need = Number(q && q.need) || 0;
    this.need = need > 0 ? need : 0;
    let pick = 1;
    if (this.need) {
      const i = RECHARGES.findIndex(a => a >= this.need);
      pick = i >= 0 ? i : RECHARGES.length - 1;
    }
    this.setData({ options: RECHARGES.map(a => ({ fen: a, text: S.yuan(a) })), pick, need: this.need, needText: S.yuan(this.need) });
  },
  onShow() { this.load(); },
  load() {
    this.setData({
      balanceText: S.yuan(S.getBalance()),
      logs: S.getCardLog().map(l => ({
        id: l.id, title: l.title, type: l.type, typeName: TYPE[l.type] || '其他', time: S.fmtTime(l.time),
        plus: l.amount > 0, sign: l.amount > 0 ? '+' : '-', abs: S.yuan(Math.abs(l.amount))
      }))
    });
  },
  onPick(e) { this.setData({ pick: Number(e.currentTarget.dataset.i) }); },
  // 充值：先付款（宿主没有支付能力时退回页内确认框），付成功才入账
  onRecharge() {
    if (this.paying) return;
    const fen = RECHARGES[this.data.pick];
    this.paying = true;
    host.pay(fen, '禾味食堂饭卡充值').then(ok => {
      this.paying = false;
      if (!ok) { wx.showToast({ title: '已取消充值', icon: 'none' }); return; }
      S.changeBalance(fen, '饭卡充值', 'recharge');
      this.load();
      wx.showToast({ title: '充值成功 +¥' + S.yuan(fen), icon: 'none' });
      // 从支付页过来、钱已经够了：回去接着付
      if (this.need && fen >= this.need) {
        this.need = 0;
        this.setData({ need: 0 });
        setTimeout(() => wx.navigateBack(), 800);
      }
    });
  },
  onRule() {
    wx.showModal({
      title: '饭卡说明',
      content: '饭卡用于禾味食堂堂食和打包消费，充值即时到账；订单取消后饭卡支付的钱立即退回饭卡。离职时可到行政服务台办理余额退还。',
      showCancel: false,
      confirmText: '知道了'
    });
  }
});
