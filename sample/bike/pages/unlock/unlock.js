// 开锁：确认车辆与计费 → 电单车先确认佩戴头盔 → 连接车锁 / 校验 / 开锁三步 → 震动提示 → 进骑行页
const geo = require('../../utils/geo.js');
const fee = require('../../utils/fee.js');
const store = require('../../utils/store.js');

const STEPS = ['连接车锁', '校验车辆状态', '打开车锁'];

Page({
  data: {
    bike: null,
    pricing: null,
    helmet: false,
    phase: 'confirm', // confirm | working | done | blocked
    step: -1,
    steps: STEPS,
    blockReason: '',
    card: null
  },

  onLoad(q) {
    const no = String(q.no || '');
    if (!/^\d{9}$/.test(no)) {
      this.setData({ phase: 'blocked', blockReason: '车辆编号不正确，请返回重新扫码或输入' });
      return;
    }
    const bike = geo.bikeInfo(no);
    const pricing = fee.PRICING[bike.type];
    const card = fee.validCard(store.wallet().cards, bike.type, Date.now());
    let phase = 'confirm', blockReason = '';
    if (store.isReported(no)) { phase = 'blocked'; blockReason = '这辆车已被报修，正在等待运维检修，请换一辆车'; }
    else if (bike.type === 'ebike' && bike.battery < 20) { phase = 'blocked'; blockReason = '车辆电量不足 20%，请换一辆车'; }
    this.setData({ bike, pricing, card, phase, blockReason });
  },

  onUnload() { clearTimeout(this._t); },

  onHelmet(e) { this.setData({ helmet: e.detail.value.length > 0 }); },

  onUnlock() {
    const b = this.data.bike;
    if (this.data.phase !== 'confirm') return;
    if (b.type === 'ebike' && !this.data.helmet) {
      wx.showToast({ title: '请先确认已佩戴安全头盔', icon: 'none' });
      return;
    }
    this.setData({ phase: 'working', step: 0 });
    const next = (i) => {
      this._t = setTimeout(() => {
        if (i < STEPS.length - 1) { this.setData({ step: i + 1 }); next(i + 1); return; }
        this.success();
      }, 650);
    };
    next(0);
  },

  success() {
    const b = this.data.bike;
    if (store.settings().vibrate) wx.vibrateShort({ type: 'heavy' });
    const center = geo.DEFAULT_CENTER;
    const start = geo.nearestPoi(center);
    const now = Date.now();
    store.saveRide({
      no: b.no, type: b.type, battery: b.battery,
      startAt: now, lastTick: now, meters: 0, heading: 35,
      lockAt: 0, lockedMs: 0,
      from: start.name,
      path: [{ latitude: center.latitude, longitude: center.longitude }]
    });
    this.setData({ phase: 'done' });
    wx.showToast({ title: '开锁成功', icon: 'success' });
    this._t = setTimeout(() => wx.redirectTo({ url: '/pages/riding/riding' }), 900);
  },

  onBack() { wx.navigateBack({ fail: () => wx.reLaunch({ url: '/pages/index/index' }) }); },
  onPricing() { wx.navigateTo({ url: '/pages/pricing/pricing' }); }
});
