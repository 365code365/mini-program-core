const app = getApp();
Page({
  data: {
    region: ['广东省', '广州市', '天河区'], code: '', city: ['浙江省', '杭州市'], time: '09:30', date: '',
    hours: Array.from({ length: 24 }, (_, i) => `${i} 时`), mins: ['00 分', '15 分', '30 分', '45 分'], pv: [9, 2], log: []
  },
  onRegion(e) {
    // detail.code：6 位区划码数组（国家统计局 2023）
    this.setData({ region: e.detail.value, code: (e.detail.code || []).join(' / ') });
    app.note(this, `region ${e.detail.value.join(' ')} code=${(e.detail.code || []).join(',')}`);
  },
  onCity(e) { this.setData({ city: e.detail.value }); app.note(this, `level=city ${e.detail.value.join(' ')}`); },
  onTime(e) { this.setData({ time: e.detail.value }); app.note(this, `time ${e.detail.value}（start 08:00 end 18:30）`); },
  onDate(e) { this.setData({ date: e.detail.value }); app.note(this, `date ${e.detail.value}`); },
  onPv(e) { this.setData({ pv: e.detail.value }); app.note(this, `picker-view value=${JSON.stringify(e.detail.value)}`); }
});
