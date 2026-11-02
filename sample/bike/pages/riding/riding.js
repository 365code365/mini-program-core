// 骑行中：每秒累加时长 / 里程 / 费用，轨迹画在地图上。临时锁车期间照常计时计费、不累计里程。
// 切到后台时 setInterval 不可靠：onHide 记下时间戳并停表，onShow 按离开的时长一次补算里程，再接着走表
const geo = require('../../utils/geo.js');
const fee = require('../../utils/fee.js');
const store = require('../../utils/store.js');

Page({
  data: {
    has: true,
    ride: null,
    name: '',
    dur: '00:00',
    km: '0.00',
    fee: '0.00',
    discount: 0,
    speed: '0.0',
    battery: 0,
    locked: false,
    cur: null,
    markers: [],
    polyline: [],
    catchUpTip: ''
  },

  onLoad() {
    const r = store.ride();
    if (!r) { this.setData({ has: false }); return; }
    this._ride = r;
    this._n = 0;
    this._card = fee.validCard(store.wallet().cards, r.type, Date.now());
    this.setData({ name: fee.PRICING[r.type].name, ride: { no: r.no, type: r.type, from: r.from } });
    if (store.settings().keepOn) wx.setKeepScreenOn({ keepScreenOn: true, fail() {} });
  },

  onShow() {
    const r = this._ride;
    if (!r) return;
    if (r.hideAt) {
      const gap = Date.now() - r.hideAt;
      r.hideAt = 0;
      if (gap > 3000) this.advance(gap, true);
    }
    r.lastTick = Date.now();
    this.render();
    clearInterval(this._timer);
    this._timer = setInterval(() => this.tick(), 1000);
  },

  onHide() {
    clearInterval(this._timer);
    this._timer = null;
    if (this._ride) { this._ride.hideAt = Date.now(); store.saveRide(this._ride); }
  },

  onUnload() {
    clearInterval(this._timer);
    if (this._ride && store.ride()) store.saveRide(this._ride);
    wx.setKeepScreenOn({ keepScreenOn: false, fail() {} });
  },

  tick() {
    const r = this._ride;
    const now = Date.now();
    this.advance(now - r.lastTick, false);
    r.lastTick = now;
    this.render();
    if (++this._n % 5 === 0) store.saveRide(r);
  },

  // 往前走 ms 毫秒：速度带一点随机起伏，方向缓慢偏转（沿街道转弯）
  advance(ms, catchUp) {
    const r = this._ride;
    if (r.lockAt || ms <= 0) return;
    const p = fee.PRICING[r.type];
    const factor = catchUp ? 0.9 : 0.85 + Math.random() * 0.25;
    const meters = p.speed * factor * ms / 1000;
    r.meters += meters;
    r.speedNow = p.speed * factor * 3.6;
    r.heading = (r.heading + (Math.random() - 0.5) * 16 + 360) % 360;
    const last = r.path[r.path.length - 1];
    r.cur = geo.offset(r.cur || last, meters, r.heading);
    r._since = (r._since || 0) + meters;
    if (r._since > 15 || catchUp) {
      r.path.push(r.cur);
      r._since = 0;
      if (r.path.length > 400) r.path.splice(1, 1);
    }
    if (catchUp) {
      const min = Math.round(ms / 60000);
      this.setData({ catchUpTip: '已补记后台骑行' + (min ? ' ' + min + ' 分钟' : ' ' + Math.round(ms / 1000) + ' 秒') + '的里程' });
      setTimeout(() => this.setData({ catchUpTip: '' }), 3000);
    }
  },

  render() {
    const r = this._ride;
    const sec = (Date.now() - r.startAt) / 1000;
    const f = fee.calc(r.type, sec);
    const dis = fee.discount(this._card, r.type, sec, f.total);
    const cur = r.cur || r.path[r.path.length - 1];
    const used = r.type === 'ebike' ? Math.round(r.meters / 1000 / 0.45) : 0;
    this.setData({
      dur: fee.fmtDur(sec),
      km: (r.meters / 1000).toFixed(2),
      fee: f.total.toFixed(2),
      discount: dis,
      speed: r.lockAt ? '0.0' : (r.speedNow || 0).toFixed(1),
      battery: Math.max(0, r.battery - used),
      locked: !!r.lockAt,
      cur,
      markers: [
        { id: 1, latitude: r.path[0].latitude, longitude: r.path[0].longitude, iconPath: '/images/marker-start.png', width: 28, height: 35, anchor: { x: 0.5, y: 1 } },
        { id: 2, latitude: cur.latitude, longitude: cur.longitude, iconPath: '/images/marker-me.png', width: 32, height: 32, rotate: Math.round(r.heading), anchor: { x: 0.5, y: 0.5 } }
      ],
      polyline: [{ points: r.path.concat([cur]), color: '#13a868', width: 6, arrowLine: true, borderColor: '#ffffff', borderWidth: 2 }]
    });
  },

  onLockTap() {
    const r = this._ride;
    if (r.lockAt) {
      r.lockedMs += Date.now() - r.lockAt;
      r.lockAt = 0;
      r.lastTick = Date.now();
      wx.vibrateShort({ type: 'medium' });
      wx.showToast({ title: '已开锁，继续骑行', icon: 'none' });
    } else {
      r.lockAt = Date.now();
      wx.showToast({ title: '已临时锁车，期间照常计费', icon: 'none' });
    }
    store.saveRide(r);
    this.render();
  },

  onEndTap() {
    wx.showModal({
      title: '确认结束骑行？',
      content: '请将车辆停入路边白色停车框内，规范停车后再结束，禁停区还车会收取调度费',
      confirmText: '结束骑行',
      confirmColor: '#13a868',
      cancelText: '继续骑',
      success: (res) => { if (res.confirm) this.finish(); }
    });
  },

  finish() {
    clearInterval(this._timer);
    const r = this._ride;
    const now = Date.now();
    if (store.settings().vibrate) wx.vibrateLong();
    const sec = Math.round((now - r.startAt) / 1000);
    const f = fee.calc(r.type, sec);
    const dis = fee.discount(this._card, r.type, sec, f.total);
    const cur = r.cur || r.path[r.path.length - 1];
    const paid = fee.round2(f.total - dis);
    const trip = {
      id: 'T' + now, no: r.no, type: r.type, startAt: r.startAt, endAt: now, sec, meters: Math.round(r.meters),
      from: r.from, to: geo.nearestPoi(cur).name, path: r.path.concat([cur]),
      start: f.start, extra: f.extra, total: f.total, discount: dis, paid,
      card: dis > 0 ? this._card.name : '',
      status: paid > 0 ? 'unpaid' : 'paid', payBy: paid > 0 ? '' : (dis > 0 ? 'card' : 'free'), paidAt: paid > 0 ? 0 : now
    };
    const trips = store.trips();
    trips.unshift(trip);
    store.saveTrips(trips);
    store.saveRide(null);
    this._ride = null;
    wx.setKeepScreenOn({ keepScreenOn: false, fail() {} });
    wx.redirectTo({ url: '/pages/settle/settle?id=' + trip.id });
  },

  onFaultTap() { wx.navigateTo({ url: '/pages/fault/fault?no=' + this.data.ride.no }); },
  goHome() { wx.reLaunch({ url: '/pages/index/index' }); }
});
