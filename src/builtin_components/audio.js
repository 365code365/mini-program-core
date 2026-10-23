// <audio>（官方 component/audio，基础库 1.6 起不再维护，仍可用）：一个 InnerAudioContext + 默认播放条。
// wx.createAudioContext(id) 拿到的就是这个组件的播放器（audio.js 的 __legacyAudio 按宿主 id 共用一个实例）。
// 事件：bindplay / bindpause / bindtimeupdate {currentTime, duration} / bindended / binderror {errMsg}
Component({
  properties: {
    src: { type: String, value: '' },
    loop: { type: Boolean, value: false },
    controls: { type: Boolean, value: false },
    poster: { type: String, value: '' },
    name: { type: String, value: '未知音频' },
    author: { type: String, value: '未知作者' }
  },
  data: { playing: false, timeText: '00:00' },
  observers: {
    src: function (v) { if (this._a) this._a.src = v; },
    loop: function (v) { if (this._a) this._a.loop = v; }
  },
  lifetimes: {
    attached: function () {
      var key = String(this.__hostId || this.id || '');
      if (!__legacyAudio[key]) __legacyAudio[key] = __makeAudio();
      var a = this._a = __legacyAudio[key], self = this;
      a.src = this.properties.src;
      a.loop = this.properties.loop;
      var fmt = function (s) { s = Math.floor(s || 0); var m = Math.floor(s / 60); s = s % 60; return (m < 10 ? '0' : '') + m + ':' + (s < 10 ? '0' : '') + s; };
      a.onPlay(function () { self.setData({ playing: true }); self.triggerEvent('play', {}); });
      a.onPause(function () { self.setData({ playing: false }); self.triggerEvent('pause', {}); });
      a.onStop(function () { self.setData({ playing: false, timeText: '00:00' }); });
      a.onEnded(function () { self.setData({ playing: false }); self.triggerEvent('ended', {}); });
      a.onError(function (e) { self.setData({ playing: false }); self.triggerEvent('error', { errMsg: e.errMsg || 'MEDIA_ERR' }); });
      a.onTimeUpdate(function () {
        var t = a.currentTime, d = a.duration;
        self.setData({ timeText: fmt(t) });
        self.triggerEvent('timeupdate', { currentTime: t, duration: d });
      });
    },
    detached: function () { if (this._a) this._a.stop(); }
  },
  methods: {
    _toggle: function () { if (!this._a) return; if (this.data.playing) this._a.pause(); else this._a.play(); }
  }
});
