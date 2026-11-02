const store = require('../../utils/store.js');

const TAGS = [
  { name: '洗得很干净', bad: false }, { name: '准时上门', bad: false }, { name: '包装用心', bad: false },
  { name: '熨烫平整', bad: false }, { name: '有异味', bad: true }, { name: '有破损', bad: true }, { name: '送回延迟', bad: true }
];

Page({
  data: {
    id: '', summary: '',
    dims: [{ key: 'clean', name: '洗护效果', value: 0 }, { key: 'service', name: '取送服务', value: 0 }, { key: 'speed', name: '时效', value: 0 }],
    overallText: '-', tagOpts: TAGS.map(t => Object.assign({ on: false }, t)), text: '', err: '', anon: false
  },

  onLoad(q) {
    const o = store.order(q.id);
    if (!o) return;
    this.id = o.id;
    const names = o.lines.map(l => l.name).slice(0, 2).join('、');
    this.setData({ id: o.id, summary: names + (o.lines.length > 2 ? ' 等' : '') + ' 共 ' + o.count + ' 件' });
  },

  // 半星评分组件 triggerEvent('change', {value}) 回来的分数
  onStar(e) {
    const i = e.currentTarget.dataset.i;
    this.setData({ ['dims[' + i + '].value']: e.detail.value, err: '' });
    const vals = this.data.dims.map(d => d.value);
    if (vals.every(v => v > 0)) {
      const avg = Math.round(vals.reduce((a, b) => a + b, 0) / vals.length * 2) / 2;
      this.setData({ overallText: avg.toFixed(1) });
    }
  },

  onTags(e) {
    const on = e.detail.value;
    this.setData({ tagOpts: this.data.tagOpts.map(t => Object.assign({}, t, { on: on.indexOf(t.name) >= 0 })) });
  },

  onText(e) { this.setData({ text: e.detail.value, err: '' }); },
  onAnon(e) { this.setData({ anon: e.detail.value }); },

  submit() {
    const vals = this.data.dims.map(d => d.value);
    if (vals.some(v => !v)) return this.setData({ err: '三项都打个分吧' });
    const text = this.data.text.trim();
    // 给了差评就请写几句原因，方便客服跟进
    if (Math.min.apply(null, vals) <= 2 && text.length < 10) return this.setData({ err: '有低分项，请写至少 10 个字说明情况' });
    const o = store.order(this.id);
    const score = Math.round(vals.reduce((a, b) => a + b, 0) / vals.length * 2) / 2;
    o.review = {
      score, dims: vals, tags: this.data.tagOpts.filter(t => t.on).map(t => t.name), text, anon: this.data.anon,
      time: store.timeStr(Date.now())
    };
    store.saveOrder(o);
    wx.showToast({ title: '感谢你的评价', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 500);
  }
});
