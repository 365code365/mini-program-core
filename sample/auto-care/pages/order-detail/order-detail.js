const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const { STORES, SERVICES, STATUS_TEXT, CANCEL_REASONS } = require('../../utils/data.js');

const STATUS_TIP = {
  pending: '请按预约时间到店，到店后点「到店签到」通知技师接车',
  serving: '技师正在为爱车服务，完工后会通知你取车',
  done: '服务已完成，评价一下这次的体验吧',
  reviewed: '感谢你的评价，期待下次为你服务',
  cancelled: '预约已取消，工位已释放'
};
const FAB = 64;      // 悬浮按钮边长（px）
const BAR = 68;      // 底部操作栏高度（px）

Page({
  data: { order: null, statusText: '', tip: '', isNew: false, areaH: 600, fabX: 300, fabY: 400, careItems: false },
  onLoad(q) {
    this.id = q.id;
    const win = wx.getWindowInfo ? wx.getWindowInfo() : wx.getSystemInfoSync();
    const areaH = win.windowHeight - BAR;
    // 悬浮按钮初始停在右下角；movable-area 撑满操作栏以上的区域，按钮可以拖到任意位置
    this.setData({ isNew: q.new === '1', areaH, fabX: win.windowWidth - FAB - 12, fabY: areaH - FAB - 24 });
    if (q.new === '1') wx.setNavigationBarTitle({ title: '预约成功' });
  },
  onShow() { this.load(); },
  load() {
    const order = db.getOrder(this.id);
    if (!order) { this.setData({ order: null }); return; }
    const care = order.items.some((it) => { const s = SERVICES.find((v) => v.id === it.id); return s && s.cat === 'care'; });
    this.setData({
      order, statusText: STATUS_TEXT[order.status], tip: STATUS_TIP[order.status], careItems: care,
      plateText: util.fmtPlate(order.plate), timeline: order.timeline.slice().reverse().map((x, i) => Object.assign({ k: 'tl' + i }, x))
    });
  },
  update(patch, text) {
    const order = Object.assign({}, this.data.order, patch);
    if (text) order.timeline = order.timeline.concat([{ t: util.stamp(), text }]);
    db.saveOrder(order);
    this.load();
  },
  cancel() {
    wx.showActionSheet({
      itemList: CANCEL_REASONS,
      success: (r) => {
        const reason = CANCEL_REASONS[r.tapIndex];
        this.update({ status: 'cancelled', cancelReason: reason }, '已取消预约：' + reason);
        wx.showToast({ title: '已取消', icon: 'success' });
      }
    });
  },
  checkIn() {
    wx.showModal({
      title: '到店签到', content: '确认已到达「' + this.data.order.storeName + '」？签到后技师会来接车。',
      success: (r) => { if (r.confirm) this.update({ status: 'serving' }, '到店签到，技师已接车开始服务'); }
    });
  },
  finish() {
    wx.showModal({
      title: '确认取车', content: '确认服务已完成并已验车？应付 ¥' + this.data.order.total + '，请在门店前台付款。',
      success: (r) => {
        if (!r.confirm) return;
        // 含保养项目：把本次保养写回爱车档案，首页提醒从今天重新算
        if (this.data.careItems) db.markCareDone(this.data.order.carId, util.ymd(new Date()));
        this.update({ status: 'done' }, '服务完成，已交车');
      }
    });
  },
  review() { wx.navigateTo({ url: '/pages/review/review?id=' + this.id }); },
  again() {
    const o = this.data.order;
    wx.navigateTo({ url: '/pages/booking/booking?storeId=' + o.storeId + '&carId=' + o.carId + '&ids=' + o.items.map((i) => i.id).join(',') });
  },
  // 悬浮「联系门店」：拖动只改位置，点按弹菜单
  onFabMove(e) { this.fabPos = e.detail; },
  contact() {
    const o = this.data.order;
    const store = STORES.find((s) => s.id === o.storeId) || STORES[0];
    wx.showActionSheet({
      itemList: ['拨打门店电话 ' + o.storePhone, '复制门店地址', '导航到门店'],
      success: (r) => {
        if (r.tapIndex === 0) util.callPhone(o.storePhone, o.storeName);
        else if (r.tapIndex === 1) wx.setClipboardData({ data: o.storeAddr });
        else util.openStoreLocation(store);
      }
    });
  },
  copyId() { wx.setClipboardData({ data: this.data.order.id }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
