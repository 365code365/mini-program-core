const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const repair = require('../../utils/repair.js');
const SERVICE_PHONE = '400-820-1234';
const FAQ = [
  { q: '如何开始充电', a: '停好车插上充电枪，在首页点「扫码充电」扫桩身二维码，或输入桩身铭牌上的编号，确认后开始。' },
  { q: '充电费用怎么算', a: '费用 = 充电量 × (电费 + 服务费)，按时段分谷 / 平 / 峰计价，电站详情里有完整的电价表。结束后自动使用最省的充电券。' },
  { q: '充满会自动停止吗', a: '会。电量到 100% 自动结束并结算；也可以随时在充电页点「结束充电」。' },
  { q: '停车费怎么减免', a: '各电站规则不同，详见电站详情的停车说明。出场时向管理员出示充电订单即可。' },
  { q: '余额能退吗', a: '充值余额可申请退回原支付账户（赠送部分不退），请联系客服办理，1~3 个工作日到账。' }
];
Page({
  data: { car: null, carCount: 0, balance: '0.00', couponCount: 0, count: 0, favCount: 0, canInvoice: 0, repairPending: 0 },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 3 });
    this.setData({
      car: store.defaultCar(),
      carCount: store.getCars().length,
      balance: store.balance().toFixed(2),
      couponCount: store.validCoupons().length,
      count: store.stats().count,
      favCount: store.getFavs().length,
      canInvoice: store.invoiceable().length,
      repairPending: repair.pending()
    });
  },
  onCars() { wx.navigateTo({ url: '/pages/vehicles/vehicles' }); },
  onWallet() { wx.switchTab({ url: '/pages/wallet/wallet' }); },
  onCoupons() { wx.navigateTo({ url: '/pages/coupons/coupons' }); },
  onOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  onRepair() { wx.navigateTo({ url: '/pages/repair/repair' + (repair.pending() ? '?tab=list' : '') }); },
  onInvoice() { wx.navigateTo({ url: '/pages/invoice/invoice' }); },
  // 收藏电站：切到找桩页的「我的收藏」筛选（switchTab 不能带参数，经 globalData 传）
  onFavs() {
    getApp().globalData.filter = 5;
    wx.switchTab({ url: '/pages/index/index' });
  },
  onHelp() {
    wx.showActionSheet({
      itemList: FAQ.map(f => f.q).concat(['拨打客服 ' + SERVICE_PHONE]),
      success: r => {
        if (r.tapIndex === FAQ.length) { host.call(SERVICE_PHONE); return; }
        const f = FAQ[r.tapIndex];
        wx.showModal({ title: f.q, content: f.a, confirmText: '知道了', cancelText: '联系客服', confirmColor: '#00B87A',
          success: m => { if (m.cancel) host.call(SERVICE_PHONE); } });
      }
    });
  }
});
