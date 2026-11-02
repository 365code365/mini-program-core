const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const flows = require('../../utils/flows.js');
const host = require('../../utils/host.js');
Page({
  data: {
    profile: {}, card: {}, body: null, bmi: {}, bookings: [], todoCount: 0, strikes: 0, remind: true, shop: {}, plan: null, orderCount: 0, workoutCount: 0
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 3 });
    store.tick();
    const profile = store.getProfile();
    const c = store.getCard();
    const body = store.latestBody();
    const up = store.upcoming();
    const pv = store.planView();
    this.setData({
      profile,
      // WXML 里 {{name[0]}} 取字符串下标在引擎里取不到，头像上的字在这里算好
      initial: profile.name.charAt(0) || '燃',
      card: { name: c.plan, expire: c.expire, left: store.cardLeftDays(), valid: store.cardValid() },
      body,
      bmi: body ? store.bmiOf(body.weight, profile.height) : {},
      todoCount: up.length,
      bookings: up.slice(0, 3).map(b => ({
        bid: b.bid, time: b.time, name: b.name, wait: b.status === 'wait', canSign: store.signState(b),
        sub: (b.date === store.today() ? '今天' : b.date.slice(5)) + ' · ' + b.room + (b.status === 'wait' ? ' · 候补第 ' + b.rank + ' 位' : '')
      })),
      strikes: store.strikes().length,
      remind: wx.getStorageSync('fit_remind') !== false,
      shop: store.currentStore(),
      plan: pv && !pv.finished ? pv.plan.name + ' · 第 ' + pv.day + ' 天' : '',
      orderCount: store.getOrders().length,
      workoutCount: store.getWorkouts().length
    });
  },
  find(e) { return store.findBooking(e.currentTarget.dataset.id); },
  onCancel(e) { flows.cancel(this.find(e)).then(() => this.onShow()); },
  onSign(e) { flows.sign(this.find(e)).then(() => this.onShow()); },
  onRemind(e) {
    wx.setStorageSync('fit_remind', e.detail.value);
    this.setData({ remind: e.detail.value });
    wx.showToast({ title: e.detail.value ? '上课前 1 小时提醒你' : '已关闭上课提醒', icon: 'none' });
  },
  goCard() { wx.navigateTo({ url: '/pages/card/card' }); },
  goBody() { wx.navigateTo({ url: '/pages/body/body' }); },
  goBookings() { wx.navigateTo({ url: '/pages/bookings/bookings' }); },
  goOrders() { wx.navigateTo({ url: '/pages/orders/orders' }); },
  goPlan() { wx.navigateTo({ url: '/pages/plan/plan' }); },
  goProfile() { wx.navigateTo({ url: '/pages/profile/profile' }); },
  goRecords() { wx.switchTab({ url: '/pages/records/records' }); },
  goClasses() { wx.switchTab({ url: '/pages/classes/classes' }); },
  contact() {
    const s = this.data.shop;
    wx.showActionSheet({
      itemList: ['拨打 ' + s.short + ' ' + s.phone, '导航到' + s.short, '切换常去门店'],
      success: r => {
        if (r.tapIndex === 0) host.call(s.phone, s.name + '（' + s.hours + '）');
        else if (r.tapIndex === 1) host.openMap(s);
        else {
          wx.showActionSheet({
            itemList: data.stores.map(x => x.name + '（' + x.dist + '）'),
            success: k => {
              store.setStore(k.tapIndex);
              this.onShow();
              wx.showToast({ title: '常去门店改为' + data.stores[k.tapIndex].short, icon: 'none' });
            }
          });
        }
      }
    });
  }
});
