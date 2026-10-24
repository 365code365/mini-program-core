const cart = require('../../utils/cart.js');

function today() {
  const d = new Date();
  return `${d.getFullYear()}-${d.getMonth() + 1}-${d.getDate()}`;
}

Page({
  data: {
    signed: false,
    plusSaved: 368.5,
    remind: true,
    assets: [
      { name: '鲜豆', value: 860 },
      { name: '优惠券', value: 5 },
      { name: '余额', value: '¥42.6' },
      { name: '收藏', value: 0 }
    ],
    orderEntries: [
      { name: '待付款', glyph: '付', color: '#ff8a3d', bg: '#fff1ea', badge: 0 },
      { name: '待收货', glyph: '收', color: '#12b76a', bg: '#e6f8ee', badge: 0 },
      { name: '待评价', glyph: '评', color: '#2a8cf0', bg: '#e8f3fe', badge: 0 },
      { name: '退款售后', glyph: '退', color: '#7a8494', bg: '#eef0f3', badge: 0 }
    ],
    menus: [
      { name: '收货地址', glyph: '址', color: '#12b76a', extra: '3 个' },
      { name: '我的收藏', glyph: '藏', color: '#ff8a00', extra: '' },
      { name: '邀请有礼', glyph: '邀', color: '#ff4d2d', extra: '得 ¥10 无门槛' },
      { name: '在线客服', glyph: '服', color: '#5a6af0', extra: '' }
    ]
  },

  onShow() {
    const favs = (wx.getStorageSync('fm_favs') || []).length;
    const orders = wx.getStorageSync('fm_order_count') || 0;
    this.setData({
      signed: wx.getStorageSync('fm_sign') === today(),
      remind: wx.getStorageSync('fm_remind') !== false,
      'assets[0].value': wx.getStorageSync('fm_beans') || 860,
      'assets[3].value': favs,
      'menus[1].extra': favs ? favs + ' 件' : '',
      'orderEntries[1].badge': orders,
      'orderEntries[0].badge': cart.get().length ? 1 : 0
    });
  },

  onSign() {
    if (this.data.signed) {
      wx.showToast({ title: '今天已签到，明天再来', icon: 'none' });
      return;
    }
    const beans = (wx.getStorageSync('fm_beans') || 860) + 20;
    wx.setStorageSync('fm_beans', beans);
    wx.setStorageSync('fm_sign', today());
    this.setData({ signed: true, 'assets[0].value': beans });
    wx.showToast({ title: '签到成功 +20 鲜豆', icon: 'none' });
  },

  onAsset(e) {
    wx.showToast({ title: `${e.currentTarget.dataset.name}明细`, icon: 'none' });
  },

  onPlus() {
    wx.showModal({
      title: 'PLUS 会员',
      content: '¥88/年：每周 2 张免运费券、会员专享价、坏单极速退款。确认续费？',
      confirmText: '续费',
      success: (res) => {
        if (res.confirm) wx.showToast({ title: '续费成功', icon: 'success' });
      }
    });
  },

  onOrders(e) {
    const name = (e.currentTarget.dataset && e.currentTarget.dataset.name) || '全部订单';
    wx.showToast({ title: `${name}：暂无更多订单`, icon: 'none' });
  },

  onMenu(e) {
    const name = e.currentTarget.dataset.name;
    if (name === '在线客服') {
      wx.showActionSheet({ itemList: ['商品有坏损', '少送 / 错送', '修改收货地址', '其他问题'] });
      return;
    }
    wx.showToast({ title: name, icon: 'none' });
  },

  onRemind(e) {
    wx.setStorageSync('fm_remind', e.detail.value);
    this.setData({ remind: e.detail.value });
  }
});
