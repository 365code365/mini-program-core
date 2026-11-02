// 考勤打卡：定位判断是否在打卡范围（300 米），连着公司 Wi-Fi 也算在范围内；月统计放 worker 里算。
const S = require('../../utils/store.js');
const ST = require('../../utils/stats.js');

const OFFICE = { name: '云协同大厦 12F', addr: '杭州市西湖区文三路 478 号', lat: 30.27758, lng: 120.13021, radius: 300 };
const OFFICE_WIFI = ['YXT-Office', 'YXT-Office-5G'];
const WEEK = ['日', '一', '二', '三', '四', '五', '六'];

function distance(a, b, c, d) {
  const R = 6371000, rad = Math.PI / 180;
  const x = Math.sin((c - a) * rad / 2) ** 2 + Math.cos(a * rad) * Math.cos(c * rad) * Math.sin((d - b) * rad / 2) ** 2;
  return Math.round(2 * R * Math.asin(Math.sqrt(x)));
}

Page({
  data: {
    office: OFFICE, clock: '', loc: { state: 'locating', text: '正在定位…' }, wifi: '', inRange: false,
    today: {}, month: '', stats: null, via: '', week: WEEK, statsLoading: true
  },
  onLoad() {
    const d = new Date();
    this.setData({ month: S.ymd(d).slice(0, 7), todayKey: S.ymd(d), maxMonth: S.ymd(d).slice(0, 7) });
    this.tick();
    this.timer = setInterval(() => this.tick(), 1000);
    this.locate();
    this.checkWifi();
    this.loadStats();
  },
  onShow() { this.setData({ today: S.attend()[S.ymd(new Date())] || {} }); },
  onUnload() { clearInterval(this.timer); ST.stop(); },
  tick() {
    const d = new Date();
    this.setData({ clock: S.hm(d) + ':' + S.pad(d.getSeconds()) });
  },
  locate() {
    this.setData({ loc: { state: 'locating', text: '正在定位…' } });
    wx.getLocation({
      type: 'gcj02',
      success: res => {
        const dist = distance(res.latitude, res.longitude, OFFICE.lat, OFFICE.lng);
        this.setData({ loc: { state: 'ok', dist, text: dist <= OFFICE.radius ? `已进入考勤范围，距公司 ${dist} 米` : `距公司 ${dist} 米，不在考勤范围` } });
        this.judge();
      },
      // 拿不到定位（桌面端 / 未授权）：按工位的默认位置判断，并写明是默认位置
      fail: () => {
        const dist = distance(OFFICE.lat + 0.0006, OFFICE.lng + 0.0004, OFFICE.lat, OFFICE.lng);
        this.setData({ loc: { state: 'default', dist, text: `无法获取定位，已按默认工位位置判断（距公司 ${dist} 米）` } });
        this.judge();
      }
    });
  },
  // Wi-Fi 只是辅助：startWifi / getConnectedWifi 失败（没有 Wi-Fi 模块、未连接）直接忽略
  checkWifi() {
    wx.startWifi({
      success: () => wx.getConnectedWifi({
        success: res => {
          const ssid = res.wifi && res.wifi.SSID;
          this.setData({ wifi: ssid && OFFICE_WIFI.indexOf(ssid) >= 0 ? ssid : '' });
          this.judge();
        },
        fail: () => {}
      }),
      fail: () => {}
    });
  },
  judge() {
    const l = this.data.loc;
    this.setData({ inRange: !!this.data.wifi || (l.dist !== undefined && l.dist <= OFFICE.radius) });
  },
  punch() {
    const t = this.data.today;
    if (t.in && t.out) return wx.showToast({ title: '今天的卡已打完', icon: 'none' });
    if (!this.data.inRange) return wx.showModal({ title: '不在考勤范围', content: '请到公司 300 米范围内或连接公司 Wi-Fi 后打卡', showCancel: false });
    const kind = t.in ? 'out' : 'in';
    const go = () => {
      const day = S.punch(kind, OFFICE.name);
      const late = kind === 'in' && day.in > '09:00';
      const early = kind === 'out' && day.out < '18:00';
      this.setData({ today: day });
      wx.vibrateShort && wx.vibrateShort({ type: 'medium' });
      wx.showToast({ title: (kind === 'in' ? '上班' : '下班') + '打卡成功' + (late ? '（迟到）' : early ? '（早退）' : ''), icon: late || early ? 'none' : 'success' });
      this.loadStats();
    };
    if (kind === 'out' && S.hm(new Date()) < '18:00') {
      wx.showModal({ title: '还没到下班时间', content: '现在打下班卡会记为早退，确定打卡吗？', confirmText: '仍要打卡', success: r => { if (r.confirm) go(); } });
    } else go();
  },
  onMonth(e) { this.setData({ month: e.detail.value }); this.loadStats(); },
  loadStats() {
    this.setData({ statsLoading: true });
    ST.compute(this.data.month, S.attend(), S.ymd(new Date())).then(r => {
      const pad = Array.from({ length: r.stats.lead }, (_, i) => ({ d: '', key: 'p' + i, st: 'pad' }));
      this.setData({ stats: r.stats, cells: pad.concat(r.stats.days), via: r.via === 'worker' ? 'Worker 线程统计' : '主线程统计（' + r.why + '）', statsLoading: false });
    });
  },
  tapDay(e) {
    const key = e.currentTarget.dataset.key;
    if (!key || key[0] === 'p') return;
    const r = S.attend()[key];
    wx.showModal({
      title: key + ' 打卡记录',
      content: r && r.in ? `上班 ${r.in}　下班 ${r.out || '未打卡'}\n地点：${r.place}` : '当天没有打卡记录',
      showCancel: false
    });
  }
});
