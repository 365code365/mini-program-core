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
    percent: 0
  },
  onLoad() {
    const r = Object.assign({ intro: '' }, wx.getStorageSync('jobs_resume') || {});
    this.setData({ r, cityIdx: Math.max(0, CITIES.indexOf(r.city)) });
    this.calc();
  },
  calc() {
    const r = this.data.r;
    const filled = FIELDS.filter(k => String(r[k] || '').trim()).length;
    this.setData({ percent: Math.round(60 + 40 * filled / FIELDS.length) });
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
    wx.setStorageSync('jobs_resume', r);
    wx.showToast({ title: '保存成功', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 800);
  }
});
