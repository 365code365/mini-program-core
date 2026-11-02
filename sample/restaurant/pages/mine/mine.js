const S = require('../../utils/store.js');
const host = require('../../utils/host.js');
const { CANTEEN } = require('../../utils/data.js');

Page({
  data: { table: '', balanceText: '0', orders: 0, spentText: '0', savedText: '0', topDish: '—', feedback: 0, phone: CANTEEN.phone },
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
    this.saved = saved;
    this.setData({
      table: S.getTable(), balanceText: S.yuan(S.getBalance()), orders: list.length,
      spentText: S.yuan(spent), savedText: S.yuan(saved), topDish: top + (max ? ' · ' + max + ' 次' : ''),
      feedback: S.getFeedback().length
    });
  },
  // 饭卡：充值与明细都在饭卡页
  openCard() { wx.navigateTo({ url: '/pages/card/card' }); },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  onSaved() {
    wx.showModal({
      title: '满减省下 ¥' + S.yuan(this.saved || 0),
      content: '每单满 30 减 5、满 60 减 12，下单时自动减。点餐页底部的进度条会提示还差多少能多减。',
      confirmText: '去点餐',
      success: r => { if (r.confirm) wx.switchTab({ url: '/pages/menu/menu' }); }
    });
  },
  changeTable() { wx.switchTab({ url: '/pages/scan/scan' }); },
  leaveTable() {
    if (!this.data.table) {
      wx.showToast({ title: '还没有入座', icon: 'none' });
      return;
    }
    wx.showModal({
      title: '离开餐桌', content: '离开后再次点餐需要重新扫码选桌', confirmText: '离开',
      success: r => { if (r.confirm) { S.setTable(''); this.refresh(); } }
    });
  },
  openHelp() { wx.navigateTo({ url: '/pages/help/help' }); },
  callCanteen() { host.call(CANTEEN.phone, '联系食堂'); },
  clearAll() {
    wx.showModal({
      title: '清除本地数据', content: '将清空桌号、购物车、订单、饭卡明细与反馈记录，饭卡余额恢复默认', confirmText: '清除',
      success: r => {
        if (!r.confirm) return;
        Object.keys(S.K).forEach(k => wx.removeStorageSync(S.K[k]));
        this.refresh();
        wx.showToast({ title: '已清除', icon: 'none' });
      }
    });
  }
});
