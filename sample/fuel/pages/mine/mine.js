const { OILS } = require('../../utils/data.js');
const store = require('../../utils/store.js');

const NET = { wifi: 'Wi-Fi', '2g': '2G', '3g': '3G', '4g': '4G', '5g': '5G', none: '无网络', unknown: '未知' };
// 新能源 8 位、燃油车 7 位；第二位是发牌机关字母
const PLATE = /^[京津沪渝冀豫云辽黑湘皖鲁新苏浙赣鄂桂甘晋蒙陕吉闽贵粤青藏川宁琼][A-HJ-NP-Z][A-HJ-NP-Z0-9]{5,6}$/;

Page({
  data: { oils: OILS, oilIndex: 1, plate: '', plateErr: '', balance: '0.00', couponCount: 0, todoInvoice: 0, titleCount: 0, monthCount: 0, savedTotal: '0.00', net: { isConnected: true }, netText: '' },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 3 });
    const recs = store.records();
    const m = store.monthKey(Date.now());
    this.setData({
      oilIndex: Math.max(0, OILS.indexOf(store.oilPref())), plate: wx.getStorageSync('fuel_plate') || '',
      balance: store.card().balance.toFixed(2), couponCount: store.coupons().filter((c) => c.status === 'unused').length,
      todoInvoice: recs.filter((r) => !r.invoice).length, titleCount: store.titles().length,
      monthCount: recs.filter((r) => store.monthKey(r.createdAt) === m).length,
      savedTotal: store.money(recs.reduce((s, r) => s + parseFloat(r.discount) + parseFloat(r.couponCut), 0))
    });
    this.onNetChange(getApp().globalData.net);
  },
  onNetChange(n) { this.setData({ net: n, netText: NET[n.networkType] || n.networkType }); },
  checkNet() {
    wx.getNetworkType({
      success: (r) => {
        this.onNetChange({ networkType: r.networkType, isConnected: r.networkType !== 'none' });
        wx.showToast({ title: r.networkType === 'none' ? '当前无网络' : `网络正常（${NET[r.networkType] || r.networkType}）`, icon: 'none' });
      }
    });
  },
  onOil(e) { const i = Number(e.detail.value); this.setData({ oilIndex: i }); store.setOilPref(OILS[i]); wx.showToast({ title: `常用油号已设为 ${OILS[i]}`, icon: 'none' }); },
  onPlate(e) {
    const v = String(e.detail.value).replace(/[·\s]/g, '').toUpperCase();
    if (v && !PLATE.test(v)) { this.setData({ plateErr: '车牌格式不对，如 浙AD12345' }); return; }
    wx.setStorageSync('fuel_plate', v);
    this.setData({ plate: v, plateErr: '' });
    if (v) wx.showToast({ title: '已保存车牌', icon: 'success' });
  },
  goCards(e) { wx.navigateTo({ url: '/pages/cards/cards?tab=' + e.currentTarget.dataset.tab }); },
  goInvoice() { wx.navigateTo({ url: '/pages/invoice/invoice' }); },
  goTitles() { wx.navigateTo({ url: '/pages/titles/titles' }); },
  callService() {
    wx.makePhoneCall({ phoneNumber: '4008605517', fail: () => wx.showModal({ title: '易加油客服', content: '客服热线 400-860-5517\n7×24 小时，加油异常可直接报订单号', showCancel: false }) });
  }
});
