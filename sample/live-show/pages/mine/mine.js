const D = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { stats: [], phone: '', viewerN: 0 },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 3 });
    const orders = store.orders();
    const phone = wx.getStorageSync('ls_phone') || '';
    this.setData({
      phone: phone ? phone.slice(0, 3) + '****' + phone.slice(7) : '未绑定手机',
      viewerN: store.viewers().length,
      stats: [
        { k: '待观演', v: orders.filter((o) => o.status === 'paid').length, to: 'wallet' },
        { k: '待支付', v: orders.filter((o) => o.status === 'unpaid').length, to: 'wallet' },
        { k: '想看', v: store.wants().length, to: 'want' },
        { k: '开售提醒', v: Object.keys(store.reminds()).length, to: 'want' }
      ]
    });
  },
  go(e) { wx.switchTab({ url: '/pages/' + e.currentTarget.dataset.to + '/' + e.currentTarget.dataset.to }); },
  viewers() { wx.navigateTo({ url: '/pages/viewer/viewer' }); },
  notice() {
    wx.showModal({ title: '购票须知', content: D.NOTICE.map((n) => n.k + '：' + n.v).join('\n'), showCancel: false, confirmText: '我知道了' });
  },
  service() {
    wx.showActionSheet({
      itemList: ['拨打客服 400-820-6688（9:00-21:00）', '复制客服邮箱'],
      success: (r) => {
        if (r.tapIndex === 0) wx.makePhoneCall({ phoneNumber: '4008206688', fail: () => wx.setClipboardData({ data: '400-820-6688' }) });
        else wx.setClipboardData({ data: 'service@xianchang-live.cn' });
      }
    });
  },
  clear() {
    wx.showModal({
      title: '清除本地数据', content: '将清除本机保存的订单、想看、提醒与观演人，且无法恢复', confirmText: '清除', confirmColor: '#ff3b5c',
      success: (r) => {
        if (!r.confirm) return;
        store.clearAll();
        wx.showToast({ title: '已清除', icon: 'success' });
        this.onShow();
      }
    });
  },
  about() {
    wx.showModal({ title: '关于现场', content: '现场 v1.4.0\n现场文化传播（上海）有限公司\n营业性演出许可证：沪市演经字〔2026〕0418 号', showCancel: false });
  },
  onShareAppMessage() { return { title: '现场：演唱会、话剧、音乐节在线选座', path: '/pages/index/index' }; }
});
