const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const orders = require('../../utils/orders.js');
const host = require('../../utils/host.js');
const vip = require('../../utils/vip.js');

const INVITE_CODE = 'BL6620';

Page({
  data: {
    vip: { active: false },
    saved: 0,
    vipLeft: 0,
    vipExpire: '',
    stats: [
      { key: 'red', name: '红包', value: 0 },
      { key: 'shop', name: '代金券', value: 0 },
      { key: 'wallet', name: '余额', value: '¥0' },
      { key: 'beans', name: '吃货豆', value: 0 }
    ],
    orderEntries: [
      { key: 'unpaid', name: '待付款', glyph: '付', color: 'linear-gradient(135deg, #ffb347, #ff7a1a)', badge: 0 },
      { key: 'going', name: '进行中', glyph: '送', color: 'linear-gradient(135deg, #7cc6ff, #2a7fff)', badge: 0 },
      { key: 'rate', name: '待评价', glyph: '评', color: 'linear-gradient(135deg, #ffa6c1, #e8487c)', badge: 0 },
      { key: 'refund', name: '退款/售后', glyph: '退', color: 'linear-gradient(135deg, #c8ced8, #7a8494)', badge: 0 }
    ],
    menus: [
      { key: 'address', name: '收货地址', glyph: '址', color: '#1677ff', extra: '' },
      { key: 'favs', name: '我的收藏', glyph: '藏', color: '#ff8a00', extra: '' },
      { key: 'reviews', name: '我的评价', glyph: '评', color: '#e8487c', extra: '' },
      { key: 'invite', name: '邀请好友得红包', glyph: '邀', color: '#ff4d2d', extra: '每邀 1 人得 ¥10' },
      { key: 'service', name: '客服中心', glyph: '服', color: '#5a6af0', extra: '7×24 小时' }
    ],
    notify: true
  },

  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    const n = orders.counts();
    const v = store.getVip();
    const vipReds = store.validCoupons('red').filter(c => c.vip);
    this.setData({
      vip: v,
      saved: orders.savedThisMonth(),
      vipLeft: vipReds.length,
      vipExpire: vipReds.length ? vipReds[0].expire.slice(5) : '',
      'stats[0].value': store.validCoupons('red').length,
      'stats[1].value': store.allCoupons().filter(c => c.kind === 'shop' && c.status === 'valid').length,
      'stats[2].value': '¥' + store.getWallet().balance,
      'stats[3].value': store.getBeans(),
      'orderEntries[0].badge': n.unpaid,
      'orderEntries[1].badge': n.going,
      'orderEntries[2].badge': n.rate,
      'menus[0].extra': store.getAddrs().length + ' 个',
      'menus[1].extra': store.getFavs().length ? store.getFavs().length + ' 家' : '',
      'menus[2].extra': n.rated ? n.rated + ' 条' : '',
      notify: store.getFlag(store.K.notify)
    });
  },

  onStat(e) {
    const key = e.currentTarget.dataset.key;
    if (key === 'red' || key === 'shop') wx.navigateTo({ url: `/pages/coupons/coupons?tab=${key}` });
    else if (key === 'wallet') wx.navigateTo({ url: '/pages/wallet/wallet' });
    else wx.navigateTo({ url: '/pages/beans/beans' });
  },

  // 会员卡：没开通 → 付款开通；开通了 → 去用会员红包
  onVip() {
    if (this.data.vip.active) {
      wx.navigateTo({ url: '/pages/coupons/coupons?tab=red' });
      return;
    }
    vip.buy().then(ok => { if (ok) this.onShow(); });
  },

  onRenew() {
    vip.buy().then(ok => { if (ok) this.onShow(); });
  },

  onOrders(e) {
    getApp().globalData.orderTab = e.currentTarget.dataset.key;
    wx.switchTab({ url: '/pages/orders/orders' });
  },

  onAllOrders() {
    getApp().globalData.orderTab = 'all';
    wx.switchTab({ url: '/pages/orders/orders' });
  },

  onMenu(e) {
    const key = e.currentTarget.dataset.key;
    if (key === 'address') wx.navigateTo({ url: '/pages/address/address' });
    else if (key === 'favs') wx.navigateTo({ url: '/pages/favorites/favorites' });
    else if (key === 'reviews') wx.navigateTo({ url: '/pages/reviews/reviews' });
    else if (key === 'service') wx.navigateTo({ url: '/pages/service/service' });
    else if (key === 'invite') this.onInvite();
  },

  // 邀请：复制邀请链接发给好友；手上有兑换码的去红包页兑换
  onInvite() {
    wx.showActionSheet({
      itemList: [`复制我的邀请链接（邀请码 ${INVITE_CODE}）`, '我有兑换码，去兑换红包'],
      success: res => {
        if (res.tapIndex === 0) {
          host.copy(`饱了么外卖：注册填邀请码 ${INVITE_CODE}，新人红包 ¥20 等你拿`);
          wx.showToast({ title: '已复制，好友首单完成后你得 ¥10 红包', icon: 'none' });
        } else {
          wx.navigateTo({ url: '/pages/coupons/coupons?tab=red&redeem=1' });
        }
      }
    });
  },

  onNotify(e) {
    store.setFlag(store.K.notify, e.detail.value);
    this.setData({ notify: e.detail.value });
    wx.showToast({ title: e.detail.value ? '订单接单、出餐、送达会通知你' : '已关闭订单状态通知', icon: 'none' });
  },

  onSetting() {
    const protect = store.getFlag(store.K.protect);
    wx.showActionSheet({
      itemList: [`下单默认号码保护：${protect ? '已开启' : '已关闭'}`, '清除缓存', '关于饱了么'],
      success: res => {
        if (res.tapIndex === 0) {
          store.setFlag(store.K.protect, !protect);
          wx.showToast({ title: protect ? '已关闭，下单时号码保护默认关' : '已开启，下单时默认隐藏真实号码', icon: 'none' });
        } else if (res.tapIndex === 1) {
          this.onClearCache();
        } else {
          wx.showModal({ title: '关于饱了么', content: '饱了么外卖 v3.8.2\n杭州饱了么网络科技有限公司\n客服热线 ' + data.SERVICE_PHONE, showCancel: false, confirmText: '知道了' });
        }
      }
    });
  },

  // 清掉本地的购物车、订单、地址、红包、余额等，回到初次打开的样子
  onClearCache() {
    wx.showModal({
      title: '清除缓存',
      content: '将清空购物车、订单记录、收货地址、红包与余额等本地数据，恢复为初始状态',
      confirmText: '清除',
      success: r => {
        if (!r.confirm) return;
        Object.keys(store.K).map(k => store.K[k]).concat([orders.K.orders]).forEach(k => wx.removeStorageSync(k));
        this.onShow();
        wx.showToast({ title: '已清除', icon: 'none' });
      }
    });
  }
});
