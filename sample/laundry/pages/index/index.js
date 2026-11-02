const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const tab = require('../../utils/tab.js');

Page({
  data: {
    banners: data.BANNERS, cats: data.CATS, city: '苏州', addrText: '', active: null,
    showCode: false, code: '', codeErr: '',
    flow: [{ t: '下单预约', s: '选衣物与时段' }, { t: '上门取件', s: '2 小时内' }, { t: '工厂洗护', s: '质检拍照' }, { t: '送回签收', s: '72 小时内' }],
    promises: [
      { t: '洗坏包赔', s: '按衣物价值最高 10 倍赔付，最高 3000 元' },
      { t: '全程可追踪', s: '取件、入厂、清洗、出厂每一步实时推送' },
      { t: '不满意免费返洗', s: '签收 72 小时内可申请一次免费返洗' }
    ]
  },

  onShow() {
    tab.sync(this, 0);
    const a = store.selectedAddr();
    // 进行中的订单（最近一单未完成、未取消）
    const o = store.orders().find(x => !x.cancelled && store.mainStatus(x) < 3);
    this.setData({
      addrText: a ? a.community + ' ' + a.room : '添加取件地址，享 2 小时上门',
      active: o ? { id: o.id, bagNo: o.bagNo, statusName: store.statusName(o), stepText: data.STEPS[o.step].text + ' · ' + data.STEPS[o.step].desc } : null
    });
  },

  goCat(e) {
    getApp().globalData.priceCat = e.currentTarget.dataset.cat;
    wx.switchTab({ url: '/pages/price/price' });
  },

  goPrice() { wx.switchTab({ url: '/pages/price/price' }); },

  goActive() { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + this.data.active.id }); },

  // 扫洗衣袋上的二维码查订单；扫不了（无相机 / 取消）就弹手动输入
  scanBag() {
    wx.scanCode({
      onlyFromCamera: false,
      scanType: ['qrCode', 'barCode'],
      success: res => this.findBag(res.result),
      fail: () => this.setData({ showCode: true, code: '', codeErr: '' })
    });
  },

  onCode(e) { this.setData({ code: e.detail.value.toUpperCase(), codeErr: '' }); },
  closeCode() { this.setData({ showCode: false }); },

  submitCode() {
    const code = this.data.code.trim();
    if (!/^JY-[A-Z0-9]{4}$/.test(code)) return this.setData({ codeErr: '格式不对，应为 JY- 加 4 位字母或数字' });
    this.findBag(code);
  },

  findBag(code) {
    const o = store.orders().find(x => x.bagNo === String(code).trim().toUpperCase());
    if (!o) {
      if (this.data.showCode) this.setData({ codeErr: '没有找到这个袋码对应的订单' });
      else wx.showToast({ title: '没有找到对应订单', icon: 'none' });
      return;
    }
    this.setData({ showCode: false });
    wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + o.id });
  }
});
