const store = require('../../utils/store.js');
// 身体数据：记体重 / 体脂（同一天记多次只留最后一次），看趋势，删记录
Page({
  data: { height: 176, latest: null, bmi: {}, diff: '', bars: [], list: [], date: '', today: '', weight: '', fat: '', formSeq: 0 },
  onShow() {
    const t = store.today();
    this.setData({ today: t, date: this.data.date || t, height: store.getProfile().height });
    this.load();
  },
  load() {
    const l = store.getBody();
    const latest = l[l.length - 1] || null;
    const first = l[0];
    const recent = l.slice(-8);
    const vs = recent.map(x => x.weight);
    const min = Math.min.apply(null, vs) - 1;
    const max = Math.max.apply(null, vs);
    this.setData({
      latest,
      bmi: latest ? store.bmiOf(latest.weight, this.data.height) : {},
      diff: latest && l.length > 1 ? (latest.weight - first.weight > 0 ? '+' : '') + (Math.round((latest.weight - first.weight) * 10) / 10) + ' kg' : '',
      bars: recent.map(x => ({ id: x.id, v: x.weight, d: Number(x.date.slice(5, 7)) + '/' + Number(x.date.slice(8)), h: max > min ? Math.round(30 + (x.weight - min) / (max - min) * 150) : 100 })),
      list: l.slice().reverse().map(x => Object.assign({}, x, { bmi: store.bmiOf(x.weight, this.data.height).v }))
    });
  },
  onDate(e) { this.setData({ date: e.detail.value }); },
  onWeight(e) { this.setData({ weight: e.detail.value }); },
  onFat(e) { this.setData({ fat: e.detail.value }); },
  save() {
    const w = Number(this.data.weight);
    const fat = this.data.fat === '' ? null : Number(this.data.fat);
    if (!(w >= 30 && w <= 200)) return wx.showToast({ title: '体重请填 30~200 kg', icon: 'none' });
    if (fat !== null && !(fat >= 3 && fat <= 60)) return wx.showToast({ title: '体脂率请填 3~60 %', icon: 'none' });
    const replaced = store.getBody().some(x => x.date === this.data.date);
    store.upsertBody({ date: this.data.date, weight: Math.round(w * 10) / 10, fat: fat === null ? null : Math.round(fat * 10) / 10 });
    // 输入框打过字，setData 清空画面不更新：换个 id 让它们重建
    this.setData({ weight: '', fat: '', formSeq: this.data.formSeq + 1 });
    this.load();
    wx.showToast({ title: replaced ? '已更新 ' + this.data.date.slice(5) + ' 的记录' : '已记录', icon: 'none' });
  },
  onItem(e) {
    const r = this.data.list.find(x => x.id === e.currentTarget.dataset.id);
    wx.showModal({
      title: '删除记录',
      content: '删除 ' + r.date + ' 的 ' + r.weight + ' kg' + (r.fat ? ' / 体脂 ' + r.fat + '%' : '') + '？',
      confirmText: '删除',
      confirmColor: '#ff3d6e',
      success: res => {
        if (!res.confirm) return;
        store.removeBody(r.id);
        this.load();
        wx.showToast({ title: '已删除', icon: 'none' });
      }
    });
  }
});
