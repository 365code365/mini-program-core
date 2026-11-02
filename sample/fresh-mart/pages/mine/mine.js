const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const orders = require('../../utils/orders.js');
const host = require('../../utils/host.js');

const HOTLINE = '400-610-8899';
const INVITE_CODE = 'XDJ8826';

Page({
  data: {
    signed: false,
    plus: { active: true, expire: '', left: 0 },
    plusSaved: 368.5,
    remind: true,
    assets: [
      { key: 'beans', name: '鲜豆', value: 860 },
      { key: 'coupons', name: '优惠券', value: 0 },
      { key: 'wallet', name: '余额', value: '¥42.6' },
      { key: 'favs', name: '收藏', value: 0 }
    ],
    orderEntries: [
      { key: 'unpaid', name: '待付款', glyph: '付', color: '#ff8a3d', bg: '#fff1ea', badge: 0 },
      { key: 'receiving', name: '待收货', glyph: '收', color: '#12b76a', bg: '#e6f8ee', badge: 0 },
      { key: 'rate', name: '待评价', glyph: '评', color: '#2a8cf0', bg: '#e8f3fe', badge: 0 },
      { key: 'after', name: '退款售后', glyph: '退', color: '#7a8494', bg: '#eef0f3', badge: 0 }
    ],
    menus: [
      { key: 'address', name: '收货地址', glyph: '址', color: '#12b76a', extra: '' },
      { key: 'favs', name: '我的收藏', glyph: '藏', color: '#ff8a00', extra: '' },
      { key: 'invite', name: '邀请有礼', glyph: '邀', color: '#ff4d2d', extra: '好友得 ¥10 无门槛' },
      { key: 'service', name: '在线客服', glyph: '服', color: '#5a6af0', extra: '8:00-22:00' }
    ]
  },

  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 3, cartCount: store.cartCount() });
    const favs = store.getFavs().length;
    const c = orders.counts();
    this.setData({
      signed: store.signed(),
      plus: store.plusState(),
      plusSaved: orders.plusSaved(),
      remind: wx.getStorageSync('fm_remind') !== false,
      'assets[0].value': store.getBeans(),
      'assets[1].value': store.validCoupons().length,
      'assets[2].value': '¥' + store.getWallet().balance,
      'assets[3].value': favs,
      'menus[0].extra': store.getAddrs().length + ' 个',
      'menus[1].extra': favs ? favs + ' 件' : '',
      'orderEntries[0].badge': c.unpaid,
      'orderEntries[1].badge': c.receiving,
      'orderEntries[2].badge': c.rate
    });
  },

  onSign() {
    const beans = store.sign();
    if (beans === null) {
      wx.showToast({ title: '今天已签到，明天再来', icon: 'none' });
      return;
    }
    this.setData({ signed: true, 'assets[0].value': beans });
    wx.showToast({ title: '签到成功 +20 鲜豆', icon: 'none' });
  },

  onAsset(e) {
    const key = e.currentTarget.dataset.key;
    wx.navigateTo({ url: `/pages/${key}/${key}` });
  },

  onPlus() {
    const p = this.data.plus;
    wx.showModal({
      title: 'PLUS 会员 ¥88/年',
      content: `每周 2 张免运费券、售后极速退款。${p.active ? `当前 ${p.expire} 到期，续费后顺延一年` : '会员已过期，续费后立即恢复权益'}，并立即再送 2 张免运费券。`,
      confirmText: '去续费',
      success: (res) => {
        if (!res.confirm || this.paying) return;
        this.paying = true;
        host.pay(88, '鲜到家 PLUS 会员 · 1 年').then(ok => {
          this.paying = false;
          if (!ok) {
            wx.showToast({ title: '已取消续费', icon: 'none' });
            return;
          }
          const expire = store.renewPlus();
          this.onShow();
          wx.showToast({ title: `已续费至 ${expire}`, icon: 'none' });
        });
      }
    });
  },

  onOrders(e) {
    const tab = (e.currentTarget.dataset && e.currentTarget.dataset.key) || 'all';
    wx.navigateTo({ url: '/pages/orders/orders?tab=' + tab });
  },

  onMenu(e) {
    const key = e.currentTarget.dataset.key;
    if (key === 'address') wx.navigateTo({ url: '/pages/address/address' });
    else if (key === 'favs') wx.navigateTo({ url: '/pages/favs/favs' });
    else if (key === 'invite') this.onInvite();
    else this.onService();
  },

  onInvite() {
    wx.showActionSheet({
      itemList: ['复制邀请口令发给好友', '邀请规则'],
      success: (res) => {
        if (res.tapIndex === 0) {
          host.copy(`【鲜到家超市】30 分钟送菜到家，新人下单领 ¥10 无门槛券，注册填我的邀请码 ${INVITE_CODE}`);
          return;
        }
        wx.showModal({
          title: '邀请规则',
          content: `好友用邀请码 ${INVITE_CODE} 注册，得 ¥10 无门槛券；好友首单签收后，你得 500 鲜豆。每月最多邀请 20 位好友。`,
          showCancel: false,
          confirmText: '知道了'
        });
      }
    });
  },

  onService() {
    const items = data.faqs.map(f => f.q).concat([`拨打客服热线 ${HOTLINE}`]);
    wx.showActionSheet({
      itemList: items,
      success: (res) => {
        if (res.tapIndex === data.faqs.length) {
          host.call(HOTLINE);
          return;
        }
        const f = data.faqs[res.tapIndex];
        // 第一条（坏单 / 少送）直接带去订单列表申请售后
        if (res.tapIndex === 0) {
          wx.showModal({
            title: f.q,
            content: f.a,
            confirmText: '去申请',
            success: r => { if (r.confirm) wx.navigateTo({ url: '/pages/orders/orders?tab=all' }); }
          });
          return;
        }
        wx.showModal({ title: f.q, content: f.a, showCancel: false, confirmText: '知道了' });
      }
    });
  },

  onRemind(e) {
    wx.setStorageSync('fm_remind', e.detail.value);
    this.setData({ remind: e.detail.value });
    wx.showToast({ title: e.detail.value ? '已开启，开抢前 3 分钟提醒你' : '已关闭秒杀开抢提醒', icon: 'none' });
  }
});
