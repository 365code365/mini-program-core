const store = require('../../utils/store.js');
const T = require('../../utils/time.js');
const voice = require('../../utils/voice.js');

const SAMPLE = '/images/photos/p1.jpg';
const TAGS = ['日常', '第一次', '出游', '节日', '萌照'];

// 两步：① camera 拍照（不行就相册，再不行用示例照片）② 写一句话、选日期、录一段语音 → 保存后进详情
Page({
  data: {
    step: 'camera', pos: 'back', flash: 'off', camError: '', forAvatar: false,
    photo: '', caption: '', date: '', today: '', start: '', tags: TAGS, tag: '日常', err: '',
    recording: false, recSec: 0, voice: null, playing: false
  },

  onLoad(q) {
    const baby = store.profile();
    const today = T.ymd(Date.now());
    this.setData({ today, date: today, start: baby.birthday, forAvatar: q.for === 'avatar' });
    if (q.photo) this.setPhoto(decodeURIComponent(q.photo));
    voice.bind(this);
  },
  onUnload() { voice.unbind(this); },
  onHide() { voice.stopPlay(this); },

  // ---------- 拍照 ----------
  onCamReady() { this.setData({ camError: '' }); },
  onCamError(e) {
    this.setData({ camError: (e && e.detail && e.detail.errMsg) || '相机不可用' });
  },
  flip() { this.setData({ pos: this.data.pos === 'back' ? 'front' : 'back' }); },
  toggleFlash() { this.setData({ flash: this.data.flash === 'off' ? 'on' : 'off' }); },

  take() {
    const ctx = wx.createCameraContext();
    ctx.takePhoto({
      quality: 'high',
      success: (res) => { wx.vibrateShort({ type: 'light' }); this.setPhoto(res.tempImagePath); },
      fail: () => {
        this.setData({ camError: '这台设备拍不了照' });
        this.fromAlbum();
      }
    });
  },

  fromAlbum() {
    wx.chooseImage({
      count: 1, sizeType: ['compressed'], sourceType: ['album', 'camera'],
      success: (res) => this.setPhoto(res.tempFilePaths[0]),
      fail: () => {
        wx.showToast({ title: '相册也打不开，先用示例照片', icon: 'none' });
        this.setPhoto(SAMPLE);
      }
    });
  },
  useSample() { this.setPhoto(SAMPLE); },

  setPhoto(p) {
    if (this.data.forAvatar) {
      const prof = store.profile();
      prof.avatar = p;
      store.set(store.K.profile, prof);
      wx.showToast({ title: '头像已更新', icon: 'success' });
      setTimeout(() => wx.navigateBack(), 600);
      return;
    }
    this.setData({ photo: p, step: 'edit' });
    wx.setNavigationBarTitle({ title: '写下这个瞬间' });
  },
  retake() {
    this.setData({ step: 'camera', photo: '' });
    wx.setNavigationBarTitle({ title: '拍照记录' });
  },

  // ---------- 填写 ----------
  onCaption(e) { this.setData({ caption: e.detail.value, err: '' }); },
  onDate(e) { this.setData({ date: e.detail.value }); },
  onTag(e) { this.setData({ tag: e.currentTarget.dataset.t }); },

  toggleRecord() { this.data.recording ? voice.stopRecord(this) : voice.startRecord(this); },
  togglePlay() { voice.togglePlay(this, this.data.voice); },
  dropVoice() { voice.stopPlay(this); this.setData({ voice: null }); },

  save() {
    const caption = this.data.caption.trim();
    if (caption.length < 2) return this.setData({ err: '写一句话记下这个瞬间吧（至少 2 个字）' });
    if (this.data.recording) voice.stopRecord(this);
    const rec = store.add(store.K.moments, {
      date: this.data.date, ts: Date.now(), photo: this.data.photo, caption, tag: this.data.tag, voice: this.data.voice
    });
    wx.vibrateShort({ type: 'light' });
    wx.redirectTo({ url: '/pages/moment/moment?id=' + rec.id + '&new=1' });
  }
});
