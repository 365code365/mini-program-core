// 进行中的那一单：预约成功 / 呼叫中 → 司机赶来 → 司机到达 → 行程中 → 到达后跳支付页。
// 状态机、取消规则、计费都在 utils/order.js；这里每秒走一拍，按阶段画地图上的车和底部卡片。
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const order = require('../../utils/order.js');
const host = require('../../utils/host.js');
const TITLE = { booked: '预约成功', waiting: '等待应答', accepted: '等待接驾', arrived: '司机已到达', ongoing: '行程中' };

// 地图上车的位置（rpx）：接驾时沿左边那条路往下开到乘客，行程中沿右边那条路往上开到终点
function mapOf(o, v) {
  if (o.stage === 'accepted') {
    const p = Math.min(1, (o.t - o.acceptT) / order.APPROACH);
    const carTop = Math.round(60 + 200 * p);
    return { carTop, lineTop: carTop + 60, lineH: Math.max(0, 346 - carTop - 60) };
  }
  if (o.stage === 'arrived') return { carTop: 262, lineTop: 0, lineH: 0 };
  if (o.stage === 'ongoing') {
    const carTop = Math.round(330 - 270 * (v.progress / 100));
    return { carTop, lineTop: 64, lineH: Math.max(0, carTop - 60) };
  }
  return { carTop: 0, lineTop: 0, lineH: 0 };
}
function headOf(o, v) {
  if (o.stage === 'booked') return { title: '预约成功', sub: v.whenText + ' 出发' };
  if (o.stage === 'accepted') return { title: '司机正在赶来', sub: '约 ' + v.etaMin + ' 分钟到达' };
  if (o.stage === 'arrived') return { title: '司机已到达', sub: '免费等待剩余 ' + v.waitLeft };
  if (o.stage === 'ongoing') return { title: '行程中', sub: '预计 ' + v.arriveText + ' 到达' };
  return { title: '', sub: '' };
}

Page({
  data: { o: null, v: {}, stage: '', head: {}, est: null, map: {}, recording: true, nearby: 12, leftKm: 0, pMask: '', msgs: [] },
  onShow() {
    this.refresh();
    this.start();
  },
  onHide() {
    this.stop();
  },
  onUnload() {
    this.stop();
    this.done = true;
  },
  start() {
    this.stop();
    if (!this.done && order.get()) this.timer = setInterval(() => this.tick(), 1000);
  },
  stop() {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
  },
  tick() {
    if (this.done) return;
    const r = order.step();
    if (r.ev === 'finished') {
      this.stop();
      this.done = true;
      wx.vibrateShort({ type: 'medium' });
      wx.redirectTo({ url: '/pages/pay/pay?id=' + r.trip.id });
      return;
    }
    if (r.ev === 'accepted' || r.ev === 'depart') {
      wx.vibrateShort({ type: 'light' });
      wx.showToast({ title: r.o.driver.name + (r.ev === 'depart' ? '已出发，正在赶来' : '已接单，正在赶来'), icon: 'none' });
    } else if (r.ev === 'arrived') {
      wx.vibrateShort({ type: 'heavy' });
      wx.showToast({ title: '司机已到达上车点，请尽快上车', icon: 'none' });
    } else if (r.ev === 'started') {
      wx.showToast({ title: r.o.autoShared ? '夜间行程已自动分享给 ' + r.o.autoShared : '行程开始，祝你一路顺风', icon: 'none' });
    }
    this.refresh(r.o);
  },
  refresh(o) {
    o = o || order.get();
    if (!o) {
      this.setData({ o: null, stage: '' });
      wx.setNavigationBarTitle({ title: '行程' });
      return;
    }
    const v = order.view(o);
    if (this.title !== o.stage) {
      this.title = o.stage;
      wx.setNavigationBarTitle({ title: TITLE[o.stage] || '行程' });
    }
    this.setData({
      o, v, stage: o.stage, head: headOf(o, v), est: order.estimate(o), map: mapOf(o, v), recording: store.recording(),
      leftKm: store.round1(Math.max(0, o.km - (v.traveled || 0))),
      // 只露最近两条，按钮不被聊天记录挤到屏幕外
      msgs: o.msgs.slice(-2),
      pMask: o.passenger ? store.maskPhone(o.passenger.phone) : ''
    });
  },
  goHome() {
    if (getCurrentPages().length > 1) wx.navigateBack();
    else wx.switchTab({ url: '/pages/index/index' });
  },
  onHome() {
    this.goHome();
  },
  // ───────── 取消：应答前 / 预约单出发前 / 接单 2 分钟内免费，之后收 ¥5，司机原因免费，行程中不能取消 ─────────
  // 弹窗开着时停表：弹窗里说的取消费就是确认后收的
  onCancel() {
    const o = order.get();
    if (!o || o.stage === 'ongoing') return;
    this.stop();
    const resume = () => this.start();
    if (o.stage === 'waiting' || o.stage === 'booked') {
      const booked = o.stage === 'booked';
      wx.showModal({
        title: booked ? '取消预约？' : '确定取消呼叫？',
        content: booked ? '司机出发前取消免费，取消后会通知司机' : '附近车辆较多，再等一会儿很快就能应答。现在取消免费',
        confirmText: booked ? '取消预约' : '确定取消',
        cancelText: '再等等',
        success: r => (r.confirm ? this.finishCancel(undefined) : resume()),
        fail: resume
      });
      return;
    }
    wx.showActionSheet({
      itemList: data.CANCEL_REASONS,
      success: r => this.confirmCancel(r.tapIndex),
      fail: resume
    });
  },
  confirmCancel(reason) {
    const o = order.get();
    if (!o) return;
    const fee = order.cancelFee(o, reason);
    const driverFault = data.DRIVER_FAULT.indexOf(reason) >= 0;
    let content = '接单 2 分钟内取消，不收取任何费用';
    if (driverFault) content = '司机原因取消，不收取取消费，平台会核实司机的情况';
    else if (fee > 0) content = (o.stage === 'arrived' ? '司机已到达上车点' : '司机接单已超过 2 分钟') + '，取消后需支付 ¥' + fee + ' 取消费，用于补偿司机空驶';
    wx.showModal({
      title: fee > 0 ? '取消需支付 ¥' + fee : '确定取消行程？',
      content,
      confirmText: '确定取消',
      cancelText: '不取消了',
      success: r => (r.confirm ? this.finishCancel(reason) : this.start()),
      fail: () => this.start()
    });
  },
  finishCancel(reason) {
    const o = order.get();
    const trip = o ? order.cancel(o, reason) : null;
    if (!trip) {
      this.start();
      return;
    }
    this.done = true;
    if (trip.status === 'unpaid') {
      wx.redirectTo({ url: '/pages/pay/pay?id=' + trip.id });
      return;
    }
    wx.showToast({ title: '已取消，未产生费用', icon: 'none' });
    // 有司机的单留一张取消记录给用户看；还没应答的直接回首页
    if (trip.driver !== '-') wx.redirectTo({ url: '/pages/trip-detail/trip-detail?id=' + trip.id });
    else setTimeout(() => this.goHome(), 500);
  },
  // ───────── 与司机 ─────────
  onCallDriver() {
    const o = order.get();
    if (o && o.driver) host.call(o.driver.phone, '联系' + o.driver.name + '（隐私号）');
  },
  onMessage() {
    if (!order.get()) return;
    wx.showActionSheet({
      itemList: data.QUICK_MSGS,
      success: r => {
        const cur = order.get();
        if (!cur) return;
        order.sendMsg(cur, r.tapIndex);
        this.refresh(cur);
      }
    });
  },
  onBoard() {
    const o = order.boardNow();
    if (!o) return;
    this.refresh(o);
    wx.showToast({ title: o.autoShared ? '夜间行程已自动分享给 ' + o.autoShared : '行程开始，祝你一路顺风', icon: 'none' });
  },
  // ───────── 安全 ─────────
  onSafety() {
    const rec = store.recording();
    wx.showActionSheet({
      itemList: ['分享行程给紧急联系人', '一键报警 110', rec ? '关闭行程录音' : '开启行程录音', '紧急联系人设置'],
      success: r => {
        if (r.tapIndex === 0) this.onShare();
        else if (r.tapIndex === 1) this.onAlarm();
        else if (r.tapIndex === 2) {
          store.setRecording(!rec);
          this.refresh();
          wx.showToast({ title: rec ? '已关闭行程录音' : '已开启录音，加密保存 7 天', icon: 'none' });
        } else wx.navigateTo({ url: '/pages/contacts/contacts' });
      }
    });
  },
  // 分享行程：发短信给全部紧急联系人（宿主不能发短信时页内确认）
  onShare() {
    const o = order.get();
    if (!o) return;
    const list = store.getContacts();
    if (!list.length) {
      wx.showModal({
        title: '还没有紧急联系人',
        content: '添加后可以一键把实时行程分享给 TA',
        confirmText: '去添加',
        success: r => {
          if (r.confirm) wx.navigateTo({ url: '/pages/contacts/contacts' });
        }
      });
      return;
    }
    const names = list.map(c => c.name);
    host.sms(list.map(c => c.phone), names, order.shareText(o)).then(ok => {
      if (!ok) return;
      const cur = order.get();
      if (cur) {
        cur.shared = true;
        order.save(cur);
      }
      this.refresh();
      wx.showToast({ title: '已分享给 ' + names.join('、'), icon: 'none' });
    });
  },
  onAlarm() {
    const o = order.get();
    const plate = o && o.driver ? o.driver.plate : '';
    wx.showModal({
      title: '一键报警',
      content: '将拨打 110，同时把你的实时位置' + (plate ? '、车牌 ' + plate : '') + ' 同步给平台安全专员',
      confirmText: '拨打 110',
      confirmColor: '#F5455C',
      success: r => {
        if (!r.confirm) return;
        const cur = order.get();
        if (cur) {
          cur.alarm = Date.now();
          order.save(cur);
        }
        host.call('110', '报警电话');
      }
    });
  },
  // ───────── 更多 ─────────
  onMore() {
    wx.showActionSheet({
      itemList: ['修改目的地', '联系客服', '查看计价规则'],
      success: r => {
        if (r.tapIndex === 0) this.onChangeDest();
        else if (r.tapIndex === 1) host.call(data.SERVICE_PHONE, '滴达客服（7×24 小时）');
        else this.onRule();
      }
    });
  },
  onChangeDest() {
    wx.navigateTo({ url: '/pages/place/place?type=change' });
  },
  onRule() {
    const o = order.get();
    if (!o) return;
    wx.showModal({ title: o.type + '计价规则', content: data.ruleText(o.typeKey), showCancel: false, confirmText: '知道了' });
  }
});
