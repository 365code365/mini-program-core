const store = require('../../utils/store.js');
const T = require('../../utils/time.js');

const BLOOD = ['A型', 'B型', 'O型', 'AB型', '暂不清楚'];
const DEFAULT_AVATAR = '/images/avatar.jpg';

Page({
  data: { f: {}, bloods: BLOOD, bloodIdx: 2, today: '', errs: {}, ageText: '' },

  onLoad() {
    this.setData({ today: T.ymd(Date.now()) });
  },
  // 从拍照页换完头像回来要重新读
  onShow() {
    const p = store.profile();
    const f = this._dirty ? Object.assign({}, this.data.f, { avatar: p.avatar }) : Object.assign({}, p);
    this.setData({ f, bloodIdx: Math.max(0, BLOOD.indexOf(f.blood)), ageText: T.age(f.birthday).label });
    wx.setNavigationBarTitle({ title: (f.name || '宝宝') + '的资料' });
  },

  onInput(e) {
    const k = e.currentTarget.dataset.k;
    this._dirty = true;
    const errs = Object.assign({}, this.data.errs);
    delete errs[k];
    this.setData({ ['f.' + k]: e.detail.value, errs });
  },
  onSex(e) { this._dirty = true; this.setData({ 'f.sex': e.detail.value }); },
  onBirthday(e) { this._dirty = true; this.setData({ 'f.birthday': e.detail.value, ageText: T.age(e.detail.value).label }); },
  onBirthTime(e) { this._dirty = true; this.setData({ 'f.birthTime': e.detail.value }); },
  onBlood(e) { this._dirty = true; const i = Number(e.detail.value); this.setData({ bloodIdx: i, 'f.blood': BLOOD[i] }); },

  changeAvatar() {
    wx.showActionSheet({
      itemList: ['拍一张', '从手机相册选择', '恢复默认头像'],
      success: (r) => {
        if (r.tapIndex === 0) {
          this._dirty = true;
          return wx.navigateTo({ url: '/pages/shoot/shoot?for=avatar' });
        }
        if (r.tapIndex === 2) return this.setAvatar(DEFAULT_AVATAR);
        wx.chooseImage({
          count: 1, sizeType: ['compressed'], sourceType: ['album'],
          success: (res) => this.setAvatar(res.tempFilePaths[0]),
          fail: () => wx.showToast({ title: '没能打开相册', icon: 'none' })
        });
      }
    });
  },
  setAvatar(p) {
    const prof = store.profile();
    prof.avatar = p;
    store.set(store.K.profile, prof);
    this.setData({ 'f.avatar': p });
  },

  save() {
    const f = this.data.f, errs = {};
    const name = String(f.name || '').trim();
    if (!name) errs.name = '请填写乳名';
    else if (name.length > 6) errs.name = '乳名最多 6 个字';
    const bw = Number(f.birthWeight), bh = Number(f.birthHeight);
    if (f.birthWeight && (isNaN(bw) || bw < 0.8 || bw > 6)) errs.birthWeight = '出生体重应在 0.8~6kg 之间';
    if (f.birthHeight && (isNaN(bh) || bh < 30 || bh > 60)) errs.birthHeight = '出生身长应在 30~60cm 之间';
    if (f.birthday > this.data.today) errs.birthday = '生日不能晚于今天';
    this.setData({ errs });
    if (Object.keys(errs).length) return;
    const profile = Object.assign({}, f, { name, fullName: String(f.fullName || '').trim() });
    // 资料与更新时间一起写（批量接口）
    wx.batchSetStorageSync([
      { key: store.K.profile, value: profile },
      { key: 'baby_profile_updated', value: Date.now() }
    ]);
    this._dirty = false;
    wx.setNavigationBarTitle({ title: name + '的资料' });
    wx.showToast({ title: '资料已保存', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 700);
  }
});
