const store = require('../../utils/store.js');
const CITIES = ['北京', '上海', '广州', '深圳', '杭州', '成都', '南京', '武汉', '西安', '厦门'];
function pad(n) { return n < 10 ? '0' + n : '' + n; }
// 昵称长度：汉字算 2，其余算 1（与常见 App 的 2~16 字符规则一致）
function nickLen(s) { let n = 0; for (let i = 0; i < s.length; i++) n += s.charCodeAt(i) > 255 ? 2 : 1; return n; }
Page({
  data: {
    colors: store.COLORS, color: '', nickname: '', nickLen: 0, nickErr: '', gender: 'male',
    genders: [{ v: 'male', t: '男' }, { v: 'female', t: '女' }, { v: 'secret', t: '保密' }],
    birthday: '', today: '', cities: CITIES, cityIdx: 0, signature: '', signLen: 0, signMax: 60, dirty: false, canSave: true, saving: false
  },
  onLoad() {
    const p = store.profile();
    const d = new Date();
    this.setData({
      color: p.color, nickname: p.nickname, initial: p.nickname.slice(0, 1), nickLen: nickLen(p.nickname), gender: p.gender, birthday: p.birthday,
      today: d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()),
      cityIdx: Math.max(0, CITIES.indexOf(p.city)), signature: p.signature, signLen: p.signature.length
    });
  },
  validate(extra) {
    const d = Object.assign({}, this.data, extra || {});
    const n = d.nickname.trim();
    let err = '';
    if (!n) err = '昵称不能为空';
    else if (nickLen(n) < 2) err = '昵称至少 2 个字符';
    else if (nickLen(n) > 16) err = '昵称最多 16 个字符（汉字算 2 个）';
    else if (/\s/.test(n)) err = '昵称不能包含空格';
    return err;
  },
  update(patch) {
    patch.dirty = true;
    const err = this.validate(patch);
    patch.nickErr = err;
    const sign = patch.signature !== undefined ? patch.signature : this.data.signature;
    patch.canSave = !err && sign.length <= this.data.signMax;
    this.setData(patch);
  },
  onNick(e) { const v = e.detail.value; this.update({ nickname: v, nickLen: nickLen(v.trim()), initial: v.trim().slice(0, 1) || '?' }); },
  pickColor(e) { this.update({ color: e.currentTarget.dataset.c }); },
  onGender(e) { this.update({ gender: e.detail.value }); },
  onBirthday(e) { this.update({ birthday: e.detail.value }); },
  onCity(e) { this.update({ cityIdx: Number(e.detail.value) }); },
  onSign(e) { const v = e.detail.value; this.update({ signature: v, signLen: v.length }); },
  save() {
    const err = this.validate();
    if (err) { this.setData({ nickErr: err }); return wx.showToast({ title: err, icon: 'none' }); }
    if (this.data.signLen > this.data.signMax) return wx.showToast({ title: '签名超出字数', icon: 'none' });
    if (this.data.saving) return;
    const p = {
      nickname: this.data.nickname.trim(), gender: this.data.gender, birthday: this.data.birthday,
      signature: this.data.signature.trim(), color: this.data.color, city: CITIES[this.data.cityIdx]
    };
    this.setData({ saving: true });
    wx.showLoading({ title: '保存中' });
    setTimeout(() => {
      wx.hideLoading();
      wx.setStorageSync('chat_profile', p);
      const ch = typeof this.getOpenerEventChannel === 'function' ? this.getOpenerEventChannel() : null;
      if (ch && ch.emit) ch.emit('saved', p);
      wx.navigateBack();
    }, 500);
  }
});
