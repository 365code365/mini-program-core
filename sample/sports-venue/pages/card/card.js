// 会员卡：储值（充多少送多少）+ 次卡（按项目，订场时抵 1 小时 / 次），购买走微信支付（失败弹模拟收银台）
const D = require('../../utils/data.js');
const F = require('../../utils/fmt.js');
const store = require('../../utils/store.js');
const pay = require('../../utils/pay.js');
const DAYS = { 'c-bad10': 180, 'c-pp20': 180, 'c-ten5': 90 };
Page({
  data: { card: null, kind: 'stored', plans: [], pick: '', records: [], times: [], buying: false },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 2 });
    this.load();
  },
  load() {
    const card = store.card();
    const plans = D.PLANS.filter((p) => p.kind === this.data.kind);
    const pick = plans.some((p) => p.id === this.data.pick) ? this.data.pick : (plans.find((p) => p.hot) || plans[0]).id;
    this.setData({
      card, plans, pick,
      times: card.times.map((t) => Object.assign({}, t, { pct: Math.round(t.left * 100 / t.total), expText: F.dateKey(new Date(t.expire)) })),
      records: card.records.slice(0, 10).map((r) => Object.assign({}, r, { at: F.dateTime(r.at), amt: r.amount > 0 ? '+¥' + r.amount : (r.amount < 0 ? '-¥' + (-r.amount) : '') }))
    });
  },
  setKind(e) { this.setData({ kind: e.currentTarget.dataset.k }); this.load(); },
  pickPlan(e) { this.setData({ pick: e.currentTarget.dataset.id }); },
  buy() {
    if (this.data.buying) return;
    const p = D.PLANS.find((x) => x.id === this.data.pick);
    this.setData({ buying: true });
    pay.wechatPay(p.pay, p.name, (ok) => {
      this.setData({ buying: false });
      if (!ok) { wx.showToast({ title: '已取消支付', icon: 'none' }); return; }
      const c = store.card();
      if (p.kind === 'stored') {
        c.balance += p.pay + p.give;
        store.addRecord(c, '充值 ' + p.name + '（赠 ¥' + p.give + '）', p.pay + p.give);
      } else {
        const exp = Date.now() + DAYS[p.id] * 86400000;
        const t = c.times.find((x) => x.sport === p.sport);
        // 同项目的次卡合并：次数累加，有效期取更晚的
        if (t) { t.left += p.times; t.total += p.times; t.expire = Math.max(t.expire, exp); }
        else c.times.push({ planId: p.id, sport: p.sport, name: p.name, left: p.times, total: p.times, expire: exp });
        store.addRecord(c, '购买 ' + p.name, 0);
      }
      store.saveCard(c);
      this.load();
      wx.showToast({ title: '购买成功', icon: 'success' });
    });
  },
  rule() {
    wx.showModal({
      title: '会员卡说明',
      content: '储值余额全场馆通用、永久有效，不可提现；次卡仅限对应项目，每次抵 1 小时场地费（含高峰），过期作废。取消订单时余额 / 次数原路退回。',
      showCancel: false, confirmText: '知道了'
    });
  },
  toBook() { wx.switchTab({ url: '/pages/index/index' }); }
});
