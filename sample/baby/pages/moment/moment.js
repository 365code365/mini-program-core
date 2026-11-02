const store = require('../../utils/store.js');
const T = require('../../utils/time.js');
const voice = require('../../utils/voice.js');

Page({
  data: { m: null, ageText: '', dateText: '', isNew: false, playing: false, isAvatar: false },

  onLoad(q) {
    this._id = q.id;
    this.setData({ isNew: q.new === '1' });
    voice.bind(this);
    this.load();
  },
  onHide() { voice.stopPlay(this); },
  onUnload() { voice.unbind(this); },

  load() {
    const m = store.list(store.K.moments).find((x) => x.id === this._id);
    if (!m) { this.setData({ m: null }); return; }
    const baby = store.profile();
    const a = T.age(baby.birthday, T.parse(m.date));
    this.setData({ m, ageText: baby.name + ' ' + a.label + ' · 第 ' + (a.days + 1) + ' 天', dateText: T.cnDate(m.date), isAvatar: baby.avatar === m.photo });
    wx.setNavigationBarTitle({ title: T.cnDate(m.date) });
  },

  preview() { wx.previewImage({ urls: [this.data.m.photo], current: this.data.m.photo }); },
  togglePlay() { voice.togglePlay(this, this.data.m.voice); },

  setAvatar() {
    const prof = store.profile();
    prof.avatar = this.data.m.photo;
    store.set(store.K.profile, prof);
    this.setData({ isAvatar: true });
    wx.showToast({ title: '已设为宝宝头像', icon: 'success' });
  },

  del() {
    wx.showModal({
      title: '删除这个瞬间？', content: '照片和语音都会从相册里移除', confirmText: '删除', confirmColor: '#f0506e',
      success: (r) => {
        if (!r.confirm) return;
        store.remove(store.K.moments, this._id);
        wx.showToast({ title: '已删除', icon: 'none' });
        setTimeout(() => this.back(), 400);
      }
    });
  },

  back() {
    if (getCurrentPages().length > 1) wx.navigateBack();
    else wx.switchTab({ url: '/pages/album/album' });
  },
  toAlbum() { wx.switchTab({ url: '/pages/album/album' }); }
});
