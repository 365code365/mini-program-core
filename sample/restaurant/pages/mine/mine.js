const S = require('../../utils/store.js');

Page({
  data: { table: '', balanceText: '0', orders: 0, spentText: '0', savedText: '0', topDish: '—', recharge: false, amounts: [5000, 10000, 20000], amountIdx: 1 },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 3 });
    this.refresh();
  },
  refresh() {
    const list = S.getOrders().filter(o => o.status === 'done' || o.status === 'paid');
    let spent = 0, saved = 0;
    const cnt = {};
    list.forEach(o => { spent += o.pay; saved += o.discount; o.items.forEach(i => { cnt[i.name] = (cnt[i.name] || 0) + i.qty; }); });
    let top = '—', max = 0;
    Object.keys(cnt).forEach(k => { if (cnt[k] > max) { max = cnt[k]; top = k; } });
    this.setData({
      table: S.getTable(), balanceText: S.yuan(S.getBalance()), orders: list.length,
      spentText: S.yuan(spent), savedText: S.yuan(saved), topDish: top + (max ? ' · ' + max + ' 次' : '')
    });
  },
  openRecharge() { this.setData({ recharge: true }); },
  // 引擎问题：点面板里的选项并 setData 后，遮罩会再收到一次 tap（见 menu.js），紧跟着的那次忽略
  innerTap() { this.innerTapAt = Date.now(); },
  closeRecharge() { if (this.innerTapAt && Date.now() - this.innerTapAt < 400) return; this.setData({ recharge: false }); },
  pickAmount(e) { this.setData({ amountIdx: Number(e.currentTarget.dataset.i) }); },
  doRecharge() {
    const add = this.data.amounts[this.data.amountIdx];
    S.setBalance(S.getBalance() + add);
    this.setData({ recharge: false });
    this.refresh();
    wx.showToast({ title: '充值成功 +' + add / 100 + ' 元', icon: 'none' });
  },
  changeTable() { wx.switchTab({ url: '/pages/scan/scan' }); },
  leaveTable() {
    if (!this.data.table) return;
    wx.showModal({
      title: '离开餐桌', content: '离开后再次点餐需要重新扫码选桌', confirmText: '离开',
      success: r => { if (r.confirm) { S.setTable(''); this.refresh(); } }
    });
  },
  clearAll() {
    wx.showModal({
      title: '清除本地数据', content: '将清空桌号、购物车与订单记录，饭卡余额恢复默认', confirmText: '清除',
      success: r => {
        if (!r.confirm) return;
        [S.K.table, S.K.cart, S.K.orders, S.K.balance].forEach(k => wx.removeStorageSync(k));
        this.refresh();
        wx.showToast({ title: '已清除', icon: 'none' });
      }
    });
  },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); }
});
