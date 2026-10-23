// 补齐的接口：EventChannel / 路由校验 / 返回询问、Worker、UDP / TCP、getRandomValues、批量存储、
// WebAudio、MediaQueryObserver、旧版画布 clip + 虚线、宿主能力的默认 fail
const { sineWav } = require('../../utils/tone.js');

const hex = (buf) => Array.prototype.map.call(new Uint8Array(buf), (b) => (b < 16 ? '0' : '') + b.toString(16)).join(' ');

Page({
  data: {
    alertOn: false,
    routeText: '点按钮试试',
    workerText: '还没算',
    netText: '还没发',
    dataText: '点按钮看返回值',
    audioText: '会出声，注意音量',
    mqText: '',
    measured: '',
    hostText: ''
  },
  onLoad() {
    const caps = ['getLocation', 'startBluetoothDevicesDiscovery', 'createWorker', 'createUDPSocket', 'worklet', 'grid-view', 'movable-view']
      .map((k) => k + '=' + wx.canIUse(k));
    this.setData({ hostText: 'canIUse：' + caps.join('  ') });
  },
  onReady() {
    // 窗口宽 ≥ 400 才算「宽屏」；observe 之后马上回一次当前状态，转屏 / 分屏变了再回
    this.mq = this.createMediaQueryObserver();
    this.mq.observe({ minWidth: 400 }, (res) => {
      const w = wx.getWindowInfo();
      this.setData({ mqText: 'createMediaQueryObserver({minWidth: 400})：matches=' + res.matches + '（窗口 ' + w.windowWidth + '×' + w.windowHeight + '）' });
    });
    this.drawLegacy();
  },
  onUnload() {
    if (this.mq) this.mq.disconnect();
    if (this.worker) this.worker.terminate();
    if (this.udp) this.udp.close();
    wx.disableAlertBeforeUnload();
  },

  // ── 路由 ──
  openChannel() {
    wx.navigateTo({
      url: '/pages/channel/channel?from=newapi',
      events: {
        fromOpened: (d) => this.setData({ routeText: '新页面回传：' + JSON.stringify(d) })
      },
      // success 在新页面 onShow 之后才回：这时 emit，新页面 onLoad 里注册的监听已经在了
      success: (res) => res.eventChannel.emit('fromOpener', { msg: '你好，新页面', at: Date.now() % 100000 })
    });
  },
  badRoute() {
    wx.navigateTo({
      url: '/pages/nope/nope',
      success: () => this.setData({ routeText: '不该成功' }),
      fail: (e) => this.setData({ routeText: 'fail：' + e.errMsg })
    });
  },
  toggleAlert() {
    if (this.data.alertOn) wx.disableAlertBeforeUnload();
    else wx.enableAlertBeforeUnload({ message: '确定离开「补齐的接口」吗？（enableAlertBeforeUnload）' });
    this.setData({ alertOn: !this.data.alertOn, routeText: this.data.alertOn ? '返回询问已关' : '返回询问已开：现在按返回键 / 导航栏返回试试' });
  },

  // ── Worker ──
  runWorker() {
    if (!this.worker) {
      this.worker = wx.createWorker('workers/fib.js');
      this.worker.onMessage((m) => this.setData({ workerText: 'Worker 回来：fib(' + m.n + ') = ' + m.value + '，用时 ' + m.ms + 'ms' }));
      this.worker.onError((e) => this.setData({ workerText: 'Worker 出错：' + e.message }));
    }
    this.setData({ workerText: '已发给 Worker（postMessage 是异步的）…' });
    this.worker.postMessage({ n: 25 });
  },

  // ── 网络 ──
  udpLoop() {
    if (!this.udp) {
      this.udp = wx.createUDPSocket();
      this.port = this.udp.bind();
      this.udp.onMessage((r) => {
        const text = String.fromCharCode.apply(null, new Uint8Array(r.message));
        this.setData({ netText: 'UDP 收到 "' + text + '"（' + r.remoteInfo.size + ' 字节，来自 ' + r.remoteInfo.address + ':' + r.remoteInfo.port + '）' });
      });
      this.udp.onError((e) => this.setData({ netText: 'UDP onError：' + e.errMsg }));
    }
    this.udp.send({ address: '127.0.0.1', port: this.port, message: 'ping ' + (Date.now() % 1000) });
    this.setData({ netText: '已发往 127.0.0.1:' + this.port + '…' });
  },
  tcpFail() {
    const s = wx.createTCPSocket();
    s.onConnect(() => this.setData({ netText: 'TCP 竟然连上了' }));
    s.onError((e) => { this.setData({ netText: 'TCP onError：' + e.errMsg }); s.close(); });
    s.connect({ address: '127.0.0.1', port: 1, timeout: 2 });
    this.setData({ netText: 'TCP 连接中…' });
  },
  localIp() {
    wx.getLocalIPAddress({
      success: (r) => this.setData({ netText: '本机 IP：' + r.localip + '（netmask ' + r.netmask + '）' }),
      fail: (e) => this.setData({ netText: 'getLocalIPAddress fail：' + e.errMsg })
    });
  },

  // ── 随机数与存储 ──
  random() {
    wx.getRandomValues({ length: 12, success: (r) => this.setData({ dataText: 'randomValues（12 字节）：' + hex(r.randomValues) }) });
  },
  batchStorage() {
    wx.batchSetStorage({
      kvList: [{ key: 'demo_a', value: 1 }, { key: 'demo_b', value: { ok: true } }],
      success: () => wx.batchGetStorage({
        keyList: ['demo_a', 'demo_b', 'demo_none'],
        success: (r) => this.setData({ dataText: 'batchGetStorage dataList：' + JSON.stringify(r.dataList) })
      })
    });
  },
  missingKey() {
    // 微信语义：key 不存在走 fail；不给回调时返回 Promise
    wx.getStorage({ key: 'demo_no_such_key' })
      .then(() => this.setData({ dataText: '不该成功' }))
      .catch((e) => this.setData({ dataText: 'Promise reject：' + e.errMsg }));
  },
  storageInfo() {
    wx.getStorageInfo({
      success: (r) => this.setData({ dataText: 'getStorageInfo：' + r.keys.length + ' 个 key，' + r.currentSize + 'KB / ' + r.limitSize + 'KB\n' + r.keys.slice(0, 6).join(', ') })
    });
  },

  // ── WebAudio ──
  ctx() { return this.audio || (this.audio = wx.createWebAudioContext()); },
  beep() {
    const ctx = this.ctx(), osc = ctx.createOscillator(), gain = ctx.createGain();
    osc.type = 'triangle';
    osc.frequency.value = 660;
    gain.gain.value = 0.6;
    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.onended = () => this.setData({ audioText: 'onended：振荡器停了（currentTime ' + ctx.currentTime.toFixed(2) + 's）' });
    osc.start();
    osc.stop(ctx.currentTime + 0.5);
    this.setData({ audioText: '振荡器 660Hz triangle，gain 0.6，state=' + ctx.state });
  },
  decodePlay() {
    const ctx = this.ctx();
    ctx.decodeAudioData(sineWav(523, 0.6, 8000)).then((buf) => {
      const src = ctx.createBufferSource();
      src.buffer = buf;
      src.connect(ctx.destination);
      src.onended = () => this.setData({ audioText: 'BufferSource 播完了' });
      src.start();
      this.setData({ audioText: 'decodeAudioData：' + buf.sampleRate + 'Hz ' + buf.numberOfChannels + ' 声道 ' + buf.duration.toFixed(2) + 's，开始播放' });
    }).catch((e) => this.setData({ audioText: 'decode 失败：' + e.errMsg }));
  },

  // ── 旧版画布：clip / setLineDash / measureText ──
  drawLegacy() {
    const c = wx.createCanvasContext('legacy', this);
    c.save();
    c.beginPath(); c.rect(0, 0, 150, 120); c.clip();
    c.setFillStyle('#07c160'); c.fillRect(10, 10, 280, 40);
    c.restore();
    c.setStrokeStyle('#1677ff'); c.setLineWidth(4); c.setLineDash([12, 8]);
    c.beginPath(); c.moveTo(10, 80); c.lineTo(290, 80); c.stroke();
    c.setLineDash([]);
    c.setFontSize(14); c.setFillStyle('#333333'); c.fillText('clip 只留左半', 160, 35);
    c.draw();
    this.setData({ measured: Math.round(c.measureText('clip 只留左半').width) + 'px' });
  },

  // ── 宿主能力 ──
  hostApi(e) {
    const api = e.currentTarget.dataset.api;
    wx[api]({
      success: () => this.setData({ hostText: api + '：宿主接管了，success' }),
      fail: (err) => this.setData({ hostText: api + ' → fail：' + err.errMsg + '\ncanIUse(' + api + ') = ' + wx.canIUse(api) })
    });
  }
});
