// 计费说明：两种车型的价格、骑行卡、停车规范与调度费
const fee = require('../../utils/fee.js');

Page({
  data: {
    types: ['bike', 'ebike'].map(k => Object.assign({ k }, fee.PRICING[k])),
    cur: 0,
    cards: fee.CARDS,
    examples: [
      { t: '骑 12 分钟', bike: fee.calc('bike', 720).total, ebike: fee.calc('ebike', 720).total },
      { t: '骑 25 分钟', bike: fee.calc('bike', 1500).total, ebike: fee.calc('ebike', 1500).total },
      { t: '骑 50 分钟', bike: fee.calc('bike', 3000).total, ebike: fee.calc('ebike', 3000).total }
    ]
  },
  onTab(e) { this.setData({ cur: Number(e.currentTarget.dataset.i) }); },
  goWallet() { wx.navigateTo({ url: '/pages/wallet/wallet' }); }
});
