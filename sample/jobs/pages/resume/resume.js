const S = require('../../utils/store.js');
const YEARS = [];
for (let y = 2026; y >= 2010; y--) YEARS.push(String(y));
const CITIES = ['杭州', '上海', '北京', '深圳', '广州', '南京', '成都'];
const FIELDS = ['name', 'age', 'phone', 'city', 'school', 'major', 'position', 'intro'];

Page({
  data: {
    r: { intro: '' },
    genders: ['男', '女'],
    edus: ['大专', '本科', '硕士', '博士'],
    exps: ['应届生', '1 年以内', '1-3 年', '3-5 年', '5-10 年', '10 年以上'],
    salaries: ['5-8K', '8-12K', '12-20K', '20-30K', '30-50K', '50K 以上'],
    cities: CITIES,
    cityIdx: 0,
    percent: 0,
    // 工作经历面板
    showWork: false,
    w: { company: '', title: '', from: 6, to: 0 },
    fromYears: YEARS,
    toYears: ['至今'].concat(YEARS),
    wSeq: 0
  },
  onLoad() {
    const r = S.getResume();
    this.setData({ r, cityIdx: Math.max(0, CITIES.indexOf(r.city)) });
    this.calc();
  },
  calc() {
    const r = this.data.r;
    // 基础分 50，8 个字段各 5 分，有工作经历再加 10
    const filled = FIELDS.filter(k => String(r[k] || '').trim()).length;
    this.setData({ percent: Math.min(100, 50 + 5 * filled + ((r.works || []).length ? 10 : 0)) });
  },
  onInput(e) {
    this.setData({ ['r.' + e.currentTarget.dataset.key]: e.detail.value });
    this.calc();
  },
  onPick(e) { this.setData({ ['r.' + e.currentTarget.dataset.key]: Number(e.detail.value) }); },
  onCity(e) { const i = Number(e.detail.value); this.setData({ cityIdx: i, 'r.city': CITIES[i] }); },
  onSwitch(e) { this.setData({ 'r.open': e.detail.value }); },
  onSave() {
    const r = this.data.r;
    if (!String(r.name || '').trim()) { wx.showToast({ title: '请填写姓名', icon: 'none' }); return; }
    if (!/^1\d{2}[\d*]{4}\d{4}$/.test(String(r.phone || ''))) { wx.showToast({ title: '手机号格式不正确', icon: 'none' }); return; }
    S.saveResume(r);
    wx.showToast({ title: '保存成功', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 800);
  },
  onAddWork() {
    this.setData({ showWork: true, w: { company: '', title: '', from: 6, to: 0 }, wSeq: this.data.wSeq + 1 });
  },
  onCloseWork() { this.setData({ showWork: false }); },
  noop() {},
  onWInput(e) { this.setData({ ['w.' + e.currentTarget.dataset.k]: e.detail.value }); },
  onWPick(e) { this.setData({ ['w.' + e.currentTarget.dataset.k]: Number(e.detail.value) }); },
  onSaveWork() {
    const w = this.data.w;
    if (!w.company.trim()) return wx.showToast({ title: '请填写公司名称', icon: 'none' });
    if (!w.title.trim()) return wx.showToast({ title: '请填写职位名称', icon: 'none' });
    const from = YEARS[w.from];
    const to = w.to === 0 ? '至今' : YEARS[w.to - 1];
    if (to !== '至今' && Number(to) < Number(from)) return wx.showToast({ title: '离职时间不能早于入职时间', icon: 'none' });
    const works = (this.data.r.works || []).slice();
    works.unshift({ id: Date.now(), company: w.company.trim(), title: w.title.trim(), from, to });
    // 工作经历直接存，不必再点「保存简历」
    const r = Object.assign({}, S.getResume(), { works });
    S.saveResume(r);
    this.setData({ 'r.works': works, showWork: false });
    this.calc();
  },
  onDelWork(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '删除工作经历', content: '确定删除这段经历吗？', confirmColor: '#F5455C',
      success: res => {
        if (!res.confirm) return;
        const works = (this.data.r.works || []).filter(x => String(x.id) !== String(id));
        S.saveResume(Object.assign({}, S.getResume(), { works }));
        this.setData({ 'r.works': works });
        this.calc();
      }
    });
  }
});
