const S = require('../../utils/store.js');

Page({
  data: { o: null, steps: [], stage: 0, fill: 0, status: '', headline: '', eta: '', items: [], canPickup: false, active: false },
  onLoad(q) {
    // 没带 id 时展示最近一笔订单
    this.id = q.id || (S.getOrders()[0] || {}).id;
    this.render();
    // 出餐进度按付款时间推进，每秒刷新一次
    this.timer = setInterval(() => this.render(), 1000);
  },
  onUnload() { clearInterval(this.timer); },
  render() {
    const o = S.getOrder(this.id);
    if (!o) return;
    const active = o.status === 'paid';
    const stage = o.status === 'done' ? 3 : (o.stage || 0);
    if (this.data.o && this.lastKey === o.status + stage && !active) return;
    let eta = '';
    if (active && stage < 2) eta = '预计 ' + Math.max(1, Math.ceil((S.STAGE_MS[2] - (Date.now() - o.paidAt)) / 1000)) + ' 秒后可取餐';
    const headline = o.status === 'canceled' ? '订单已取消' : o.status === 'unpaid' ? '订单待支付' :
      stage >= 3 ? '已取餐，祝您用餐愉快' : stage === 2 ? '餐品已备好，请凭取餐码取餐' : stage === 1 ? '后厨正在制作' : '商家已接单';
    const steps = S.STAGES.concat(['已取餐']).map((t, i) => ({ t, done: i <= stage, cur: i === stage }));
    const key = o.status + stage;
    const patch = { eta };
    if (key !== this.lastKey) {
      this.lastKey = key;
      Object.assign(patch, {
        o: {
          no: o.no, table: o.table, dine: o.dine, remark: o.remark || '无', tableware: o.tableware === 0 ? '不需要' : o.tableware + ' 份',
          time: S.fmtTime(o.createdAt), payText: S.yuan(o.pay), discountText: S.yuan(o.discount), payMethod: o.payMethod || '—',
          status: o.status, cancelReason: o.cancelReason || ''
        },
        items: o.items.map(i => Object.assign({}, i, { lineText: S.yuan(i.unit * i.qty) })),
        steps, stage, fill: Math.min(100, Math.round(stage / 3 * 100)), headline,
        canPickup: o.status === 'paid' && stage === 2, active
      });
      if (stage === 2 && o.status === 'paid' && !this.notified) { this.notified = true; wx.vibrateShort && wx.vibrateShort({}); }
    }
    if (patch.eta !== this.data.eta || patch.o) this.setData(patch);
  },
  pickup() {
    wx.showModal({
      title: '确认取餐', content: '请确认已从取餐口拿到全部餐品', confirmText: '已取餐',
      success: r => {
        if (!r.confirm) return;
        S.updateOrder(this.id, { status: 'done', stage: 2 });
        this.render();
      }
    });
  },
  urge() { wx.showToast({ title: '已提醒后厨加急', icon: 'none' }); },
  goPay() { wx.redirectTo({ url: '/pages/pay/pay?id=' + this.id }); },
  again() { S.reorder(S.getOrder(this.id)); wx.switchTab({ url: '/pages/menu/menu' }); }
});
