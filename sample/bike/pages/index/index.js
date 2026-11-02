// 首页：全屏地图找车。车辆 markers、我的位置（指南针朝向）、定位 / 刷新、扫码开锁（扫码失败手输车号）
const geo = require('../../utils/geo.js');
const fee = require('../../utils/fee.js');
const store = require('../../utils/store.js');

const WEAK = ['none', '2g', '3g'];

Page({
  data: {
    center: { latitude: geo.DEFAULT_CENTER.latitude, longitude: geo.DEFAULT_CENTER.longitude },
    me: { latitude: geo.DEFAULT_CENTER.latitude, longitude: geo.DEFAULT_CENTER.longitude },
    scale: 16,
    markers: [],
    bikes: [],
    selected: null,
    nearCount: 0,
    nearest: 0,
    locTip: '',
    weak: false,
    netType: 'wifi',
    ride: null,
    unpaid: null,
    showInput: false,
    inputNo: '',
    inputErr: ''
  },

  onLoad() {
    this._dir = 0;
    this.locate(true);
    wx.getNetworkType({
      success: (res) => this.setNet(res.networkType),
      fail: () => this.setNet('unknown')
    });
    this._onNet = (res) => this.setNet(res.isConnected === false ? 'none' : res.networkType);
    wx.onNetworkStatusChange(this._onNet);
  },

  onShow() {
    this.refreshStatus();
    this._statusTimer = setInterval(() => this.refreshStatus(), 1000);
    this.startCompass();
  },

  onHide() { this.stopTimers(); },
  onUnload() {
    this.stopTimers();
    wx.offNetworkStatusChange && wx.offNetworkStatusChange(this._onNet);
  },

  stopTimers() {
    clearInterval(this._statusTimer);
    this._statusTimer = null;
    if (this._onCompass) {
      wx.offCompassChange && wx.offCompassChange(this._onCompass);
      wx.stopCompass({ fail() {} });
      this._onCompass = null;
    }
  },

  setNet(type) {
    this.setData({ netType: type, weak: WEAK.indexOf(type) >= 0 });
  },

  // 骑行中 / 待支付横幅（每秒刷新骑行时长）
  refreshStatus() {
    const r = store.ride();
    const unpaid = store.unpaidTrip();
    let ride = null;
    if (r) {
      const sec = (Date.now() - r.startAt) / 1000;
      ride = { dur: fee.fmtDur(sec), fee: fee.calc(r.type, sec).total.toFixed(2), locked: !!r.lockAt };
    }
    const u = unpaid ? { id: unpaid.id, total: unpaid.total.toFixed(2) } : null;
    if (JSON.stringify(ride) !== JSON.stringify(this.data.ride) || JSON.stringify(u) !== JSON.stringify(this.data.unpaid)) {
      this.setData({ ride, unpaid: u });
    }
  },

  // 定位：失败用默认城市（杭州武林广场）
  locate(first) {
    wx.getLocation({
      type: 'gcj02',
      success: (res) => {
        const p = { latitude: res.latitude, longitude: res.longitude };
        this.setData({ me: p, center: p, locTip: '' });
        this.loadBikes(p);
      },
      fail: () => {
        const p = { latitude: geo.DEFAULT_CENTER.latitude, longitude: geo.DEFAULT_CENTER.longitude };
        this.setData({ me: p, center: p, locTip: '定位未开启，已显示' + geo.DEFAULT_CENTER.city + geo.DEFAULT_CENTER.place + '附近车辆' });
        this.loadBikes(p);
        if (!first) wx.showToast({ title: '定位失败，请在设置中开启位置权限', icon: 'none' });
      }
    });
  },

  // 指南针：「我」的箭头朝向。设备不支持就忽略
  startCompass() {
    if (this._onCompass) return;
    this._onCompass = (res) => {
      const d = Math.round(res.direction || 0);
      if (Math.abs(d - this._dir) < 5) return;
      this._dir = d;
      this.buildMarkers();
    };
    wx.startCompass({
      success: () => wx.onCompassChange(this._onCompass),
      fail: () => { this._onCompass = null; }
    });
  },

  loadBikes(center) {
    const bikes = geo.genBikes(center).map(b => Object.assign(b, { dist: geo.distance(this.data.me, b) }));
    bikes.sort((a, b) => a.dist - b.dist);
    this.setData({ bikes, selected: null, nearCount: bikes.length, nearest: bikes.length ? bikes[0].dist : 0 });
    this.buildMarkers();
  },

  buildMarkers() {
    const sel = this.data.selected;
    const markers = this.data.bikes.map(b => {
      const on = sel && sel.id === b.id;
      const m = {
        id: b.id, latitude: b.latitude, longitude: b.longitude,
        iconPath: b.type === 'ebike' ? '/images/marker-ebike.png' : '/images/marker-bike.png',
        width: on ? 40 : 30, height: on ? 50 : 38, anchor: { x: 0.5, y: 1 }
      };
      if (on) m.callout = { content: '距你 ' + b.dist + ' 米', display: 'ALWAYS', padding: 6, borderRadius: 6, fontSize: 12, bgColor: '#ffffff', color: '#1f2329' };
      return m;
    });
    markers.push({ id: 0, latitude: this.data.me.latitude, longitude: this.data.me.longitude, iconPath: '/images/marker-me.png', width: 36, height: 36, rotate: this._dir, anchor: { x: 0.5, y: 0.5 } });
    this.setData({ markers });
  },

  onMarkerTap(e) {
    const id = Number(e.detail.markerId);
    if (!id) return;
    const b = this.data.bikes.find(x => x.id === id);
    if (!b) return;
    this.setData({ selected: Object.assign({ walk: Math.max(1, Math.round(b.dist / 80)) }, b) });
    this.buildMarkers();
  },

  onMapTap() {
    if (!this.data.selected) return;
    this.setData({ selected: null });
    this.buildMarkers();
  },

  // 拖动地图停下来：按新的中心重新拉附近车辆
  onRegionChange(e) {
    if (e.type !== 'end' || (e.causedBy !== 'drag' && e.causedBy !== 'gesture' && e.causedBy !== 'scale')) return;
    this.map().getCenterLocation({
      success: (res) => this.loadBikes({ latitude: res.latitude, longitude: res.longitude })
    });
  },

  map() {
    if (!this._map) this._map = wx.createMapContext('map', this);
    return this._map;
  },

  onLocateTap() {
    this.map().moveToLocation({
      latitude: this.data.me.latitude,
      longitude: this.data.me.longitude,
      fail: () => this.setData({ center: { latitude: this.data.me.latitude, longitude: this.data.me.longitude }, scale: 16 })
    });
    this.locate(false);
  },

  onRefreshTap() {
    const done = (c) => {
      this.loadBikes(c);
      wx.showToast({ title: '附近有 ' + this.data.nearCount + ' 辆可用车', icon: 'none' });
    };
    this.map().getCenterLocation({
      success: (res) => done({ latitude: res.latitude, longitude: res.longitude }),
      fail: () => done(this.data.center)
    });
  },

  onRingTap() {
    const b = this.data.selected;
    wx.vibrateShort({ type: 'light' });
    wx.showToast({ title: '已让 ' + b.no.slice(-4) + ' 号车响铃，请留意铃声', icon: 'none' });
  },

  // 扫码开锁：先查有没有没结束 / 没支付的行程
  onScanTap() {
    if (store.ride()) { wx.navigateTo({ url: '/pages/riding/riding' }); return; }
    const unpaid = store.unpaidTrip();
    if (unpaid) {
      wx.showModal({
        title: '有一笔行程待支付', content: '支付上一笔行程 ¥' + unpaid.total.toFixed(2) + ' 后才能继续用车',
        confirmText: '去支付', confirmColor: '#13a868',
        success: (r) => { if (r.confirm) wx.navigateTo({ url: '/pages/settle/settle?id=' + unpaid.id }); }
      });
      return;
    }
    const prefill = this.data.selected ? this.data.selected.no : '';
    wx.scanCode({
      onlyFromCamera: true,
      scanType: ['qrCode', 'barCode'],
      success: (res) => {
        const m = String(res.result || '').match(/\d{9}/);
        if (m) this.goUnlock(m[0]);
        else wx.showToast({ title: '这不是青桔车辆的二维码', icon: 'none' });
      },
      fail: (err) => {
        if (/cancel/.test(err.errMsg || '')) return;
        this.setData({ showInput: true, inputNo: prefill, inputErr: '' });
      }
    });
  },

  onInput(e) { this.setData({ inputNo: e.detail.value, inputErr: '' }); },
  onInputCancel() { this.setData({ showInput: false }); },
  onInputConfirm() {
    const no = String(this.data.inputNo).trim();
    if (!/^\d{9}$/.test(no)) { this.setData({ inputErr: '请输入车身二维码下方的 9 位车辆编号' }); return; }
    if (no.charAt(0) !== '1' && no.charAt(0) !== '8') { this.setData({ inputErr: '未找到车辆 ' + no + '，请核对后重新输入' }); return; }
    this.setData({ showInput: false });
    this.goUnlock(no);
  },

  goUnlock(no) { wx.navigateTo({ url: '/pages/unlock/unlock?no=' + no }); },
  goRide() { wx.navigateTo({ url: '/pages/riding/riding' }); },
  goPay() { wx.navigateTo({ url: '/pages/settle/settle?id=' + this.data.unpaid.id }); },
  go(e) { wx.navigateTo({ url: e.currentTarget.dataset.url }); },
  noop() {}
});
