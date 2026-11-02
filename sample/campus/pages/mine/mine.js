// 我的：学籍信息、上课提醒开关（switch，存 storage）、借阅 / 校园卡概览、清除缓存
const store = require('../../utils/store.js');
const kit = require('../../utils/kit.js');
Page({
  data: { remind: true, remindMin: 15, borrowCount: 0, balance: '0.00', info: {} },
  onShow() {
    store.syncTab(this, 3);
    const sys = wx.getSystemInfoSync();
    this.setData({
      remind: kit.get('remind', true), remindMin: kit.get('remindMin', 15), borrowCount: store.borrows().length,
      balance: kit.money(store.card().balance), info: { model: sys.model, system: sys.system, sdk: sys.SDKVersion }
    });
  },
  onRemind(e) {
    kit.set('remind', e.detail.value);
    this.setData({ remind: e.detail.value });
    kit.toast(e.detail.value ? '将在上课前 ' + this.data.remindMin + ' 分钟提醒' : '已关闭上课提醒');
  },
  pickMin() {
    const opts = [5, 10, 15, 30];
    wx.showActionSheet({ itemList: opts.map(m => '提前 ' + m + ' 分钟'), success: r => { kit.set('remindMin', opts[r.tapIndex]); this.setData({ remindMin: opts[r.tapIndex] }); } });
  },
  go(e) { wx.navigateTo({ url: '/pages/' + e.currentTarget.dataset.p + '/' + e.currentTarget.dataset.p }); },
  copyId() { wx.setClipboardData({ data: '2024013588' }); },
  clear() {
    kit.confirm('清除缓存', '将清除已读记录、借阅与校园卡本地数据（演示数据会恢复默认）。').then(ok => {
      if (!ok) return;
      wx.clearStorageSync();
      kit.toast('已清除');
      this.onShow();
    });
  }
});
