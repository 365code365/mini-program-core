// 补齐的官方组件：page-meta / navigation-bar、page-container、root-portal、match-media、audio、editor、
// 原生组件占位、cover-view / cover-image、span
const { sineWav } = require('../../utils/tone.js');

Page({
  data: {
    navTitle: '补齐的官方组件',
    pcShow: false,
    pcPos: 'bottom',
    pcText: '还没弹过',
    portal: false,
    winW: 0,
    toneSrc: '',
    auText: '点封面或按钮播放',
    edText: '编辑器就绪后可以点按钮'
  },
  onLoad() {
    this.setData({
      winW: wx.getWindowInfo().windowWidth,
      // 2 秒的 440Hz 正弦波：没有音频素材，现场合成
      toneSrc: wx.createBufferURL(sineWav(440, 2))
    });
  },
  onUnload() {
    if (this.data.toneSrc) wx.revokeBufferURL(this.data.toneSrc);
  },

  // ── page-container ──
  openPc(e) { this.setData({ pcPos: e.currentTarget.dataset.p, pcShow: true }); },
  closePc() { this.setData({ pcShow: false }); },
  onPcEnter() { this.setData({ pcText: 'enter：弹出来了（现在按返回键只会关它）' }); },
  // 返回键关掉时组件自己收起，show 还是 true：在 afterleave 里同步回 false，下次才能再弹
  onPcLeave() { this.setData({ pcShow: false, pcText: 'afterleave：关掉了' }); },

  // ── root-portal ──
  togglePortal() { this.setData({ portal: !this.data.portal }); },

  // ── audio ──
  auPlay() { wx.createAudioContext('au').play(); },
  auPause() { wx.createAudioContext('au').pause(); },
  onAuPlay() { this.setData({ auText: 'bindplay：在播' }); },
  onAuPause() { this.setData({ auText: 'bindpause：暂停了' }); },
  onAuEnded() { this.setData({ auText: 'bindended：播完了' }); },
  onAuError(e) { this.setData({ auText: 'binderror：' + e.detail.errMsg }); },

  // ── editor ──
  onEdReady() {
    wx.createSelectorQuery().select('#ed').context((res) => {
      this.ed = res && res.context;
      this.setData({ edText: this.ed ? 'bindready：拿到了 EditorContext' : '没拿到 EditorContext' });
    }).exec();
  },
  onEdInput(e) { this.setData({ edText: 'bindinput：' + JSON.stringify(e.detail.text) }); },
  edSet() { if (this.ed) this.ed.setContents({ html: '<p>第一段</p><p>第二段 &amp; 特殊字符</p>' }); },
  edInsert() { if (this.ed) this.ed.insertText({ text: '【插入】' }); },
  edGet() {
    if (!this.ed) return;
    this.ed.getContents({ success: (r) => this.setData({ edText: 'getContents\nhtml: ' + r.html + '\ntext: ' + JSON.stringify(r.text) }) });
  },
  edClear() { if (this.ed) this.ed.clear(); }
});
