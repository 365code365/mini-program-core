const store = require('../../utils/store.js');
const CAPS = [200, 500, 1000];
// 自动缴费：按户号开通（签约代扣），出账后不超过单笔上限的账单自动扣款。
// 开关用自己画的 view：<switch> 在用户点了「取消」之后要拨回去，而 checked 没变时 setData 不会重画它。
Page({
  data: { list: [], caps: CAPS, cap: 500, onCount: 0 },
  onShow() { this.refresh(); },
  refresh() {
    const auto = store.getAuto();
    const list = store.getAccounts().map(store.view).map(a => Object.assign(a, { on: auto.ids.indexOf(a.id) >= 0 }));
    this.setData({ list, cap: auto.cap, onCount: list.filter(a => a.on).length });
  },
  onCap(e) {
    const auto = store.getAuto();
    auto.cap = Number(e.currentTarget.dataset.v);
    store.saveAuto(auto);
    this.refresh();
    wx.showToast({ title: '单笔上限 ¥' + auto.cap, icon: 'none' });
  },
  toggle(e) {
    const id = String(e.currentTarget.dataset.id);
    const a = this.data.list.find(x => x.id === id);
    const auto = store.getAuto();
    if (a.on) {
      auto.ids = auto.ids.filter(x => x !== id);
      store.saveAuto(auto);
      this.refresh();
      wx.showToast({ title: '已关闭' + a.tname + '自动缴费', icon: 'none' });
      return;
    }
    const now = a.owe > 0 && a.owe <= auto.cap;
    wx.showModal({
      title: '开通' + a.tname + '自动缴费',
      content: '户号 ' + a.noTx + '（' + a.tag + '）\n出账后自动从微信支付扣款，单笔上限 ¥' + auto.cap + '，超出上限会提醒你手动缴费。' + (now ? '\n当前欠费 ¥' + a.oweTx + ' 将立即扣款。' : ''),
      confirmText: '同意开通',
      confirmColor: '#14a06f',
      success: r => {
        if (!r.confirm) return;
        auto.ids.push(id);
        store.saveAuto(auto);
        if (now) store.settle(id, a.owe, { way: '微信支付', auto: true });
        this.refresh();
        wx.showToast({ title: now ? '已开通，已自动缴清 ¥' + a.oweTx : '已开通自动缴费', icon: 'none' });
      }
    });
  }
});
