// 底部迷你播放条：订阅全局播放器（utils/player.js），点条身进播放页，点按钮播放 / 暂停 / 下一首。
// 进度只在整数百分比变了才 setData：onTimeUpdate 一秒好几次，每页一份迷你条，别每次都刷
const player = require('../../utils/player.js');

Component({
  properties: {
    // 页面底部是原生 tabBar 时贴底；非 tab 页同样贴底（tab 页的 window 本身就在 tabBar 之上）
    bottom: { type: Number, value: 0 }
  },
  data: { song: null, playing: false, waiting: false, progress: 0 },
  lifetimes: {
    attached() {
      this.unsub = player.subscribe((s) => {
        const progress = s.duration ? Math.min(100, Math.round((s.current / s.duration) * 100)) : 0;
        if (s.song === this.data.song && s.playing === this.data.playing && s.waiting === this.data.waiting && progress === this.data.progress) return;
        this.setData({ song: s.song, playing: s.playing, waiting: s.waiting, progress });
      });
    },
    detached() {
      if (this.unsub) this.unsub();
    }
  },
  methods: {
    onOpen() { wx.navigateTo({ url: '/pages/player/player' }); },
    onToggle() { player.toggle(); },
    onNext() { player.next(); }
  }
});
