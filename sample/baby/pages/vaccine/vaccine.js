const store = require('../../utils/store.js');
const T = require('../../utils/time.js');
const V = require('../../utils/vaccines.js');

const CLINICS = ['西湖区北山街道社区卫生服务中心', '西湖区文新街道社区卫生服务中心', '浙江大学医学院附属儿童医院 预防接种门诊'];

Page({
  data: { filter: 'todo', groups: [], doneCount: 0, total: 0, percent: 0, open: '', today: '', birthday: '', clinic: CLINICS[0] },

  onLoad() {
    this.setData({ today: T.ymd(Date.now()), clinic: store.get('baby_clinic', CLINICS[0]) });
  },
  onShow() { this.load(); },

  load() {
    const baby = store.profile();
    const done = store.get(store.K.vaccines, {});
    const today = T.ymd(Date.now());
    let doneCount = 0;
    const groups = [];
    V.LIST.forEach((v) => {
      const due = T.addMonths(baby.birthday, v.month);
      const rec = done[v.code];
      const left = T.daysBetween(today, due);
      let status, statusText;
      if (rec) { status = 'done'; statusText = rec.date + ' 已接种'; doneCount++; }
      else if (left < 0) { status = 'over'; statusText = '已逾期 ' + (-left) + ' 天'; }
      else if (left <= 30) { status = 'soon'; statusText = left === 0 ? '今天可接种' : left + ' 天后可接种'; }
      else { status = 'later'; statusText = '建议 ' + due; }
      const show = this.data.filter === 'all' || (this.data.filter === 'done' ? !!rec : !rec);
      if (!show) return;
      const label = V.ageLabel(v.month);
      let g = groups.find((x) => x.label === label);
      if (!g) { g = { label, due, items: [] }; groups.push(g); }
      g.items.push(Object.assign({}, v, { status, statusText, due }));
    });
    const total = V.LIST.length;
    this.setData({ groups, doneCount, total, percent: Math.round(doneCount / total * 100), birthday: baby.birthday });
  },

  setFilter(e) { this.setData({ filter: e.currentTarget.dataset.f, open: '' }, () => this.load()); },

  toggle(e) {
    const code = e.currentTarget.dataset.code;
    this.setData({ open: this.data.open === code ? '' : code });
  },

  // picker mode="date"：选了日期就记为已接种
  markDone(e) {
    const code = e.currentTarget.dataset.code;
    const date = e.detail.value;
    const done = store.get(store.K.vaccines, {});
    done[code] = { code, date };
    store.set(store.K.vaccines, done);
    wx.vibrateShort({ type: 'medium' });
    wx.showToast({ title: '已记录接种', icon: 'success' });
    this.setData({ open: '' });
    this.load();
  },

  undo(e) {
    const { code, name } = e.currentTarget.dataset;
    wx.showModal({
      title: '撤销接种记录', content: '把「' + name + '」改回未接种？',
      success: (r) => {
        if (!r.confirm) return;
        const done = store.get(store.K.vaccines, {});
        delete done[code];
        store.set(store.K.vaccines, done);
        this.load();
      }
    });
  },

  chooseClinic() {
    wx.showActionSheet({
      itemList: CLINICS,
      success: (r) => {
        const clinic = CLINICS[r.tapIndex];
        store.set('baby_clinic', clinic);
        this.setData({ clinic });
        wx.showToast({ title: '已设为常去门诊', icon: 'none' });
      }
    });
  },

  callClinic() {
    wx.makePhoneCall({
      phoneNumber: '0571-87061234',
      fail: () => wx.showModal({ title: '门诊电话', content: this.data.clinic + '\n0571-87061234\n（工作日 8:00-11:30、13:30-16:30）', showCancel: false })
    });
  }
});
