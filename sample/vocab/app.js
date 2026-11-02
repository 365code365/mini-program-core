const store = require('./utils/store.js');

App({
  globalData: { audio: null },
  onLaunch() {
    // 第一次打开：放一份「已经背了一周多」的学习记录，复习、日历、统计页才有东西看
    store.ensureSeed();
  },
  // 全局一个 InnerAudioContext：发音、答对 / 答错提示音共用，切换 src 即可
  play(src) {
    if (!this.globalData.audio) {
      const a = wx.createInnerAudioContext();
      a.onError((e) => console.log('音频播放失败', e && e.errMsg));
      this.globalData.audio = a;
    }
    const a = this.globalData.audio;
    a.stop();
    a.src = src;
    a.play();
  },
  speak(word) {
    this.play('/audio/w/' + word + '.mp3');
  }
});
