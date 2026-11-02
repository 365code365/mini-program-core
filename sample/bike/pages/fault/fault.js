// 故障上报：车号 + 故障部位（多选）+ 描述 + 照片 → 提交生成工单；已报修的车不能再开锁
const geo = require('../../utils/geo.js');
const store = require('../../utils/store.js');

const PARTS = {
  bike: ['车铃', '刹车', '车锁', '链条', '脚蹬', '车座', '轮胎', '车把', '二维码损坏'],
  ebike: ['车铃', '刹车', '车锁', '电池没电', '电机无助力', '车座', '轮胎', '车把', '二维码损坏', '头盔丢失']
};
const STATUS = { pending: '待处理', fixing: '运维处理中', done: '已修复' };

Page({
  data: {
    no: '',
    type: 'bike',
    parts: [],
    picked: [],
    desc: '',
    photos: [],
    place: '',
    err: '',
    submitted: null,
    history: []
  },

  onLoad(q) {
    const no = q.no || '';
    this.setType(no);
    this.setData({ no });
    wx.getLocation({
      type: 'gcj02',
      success: (res) => this.setData({ place: geo.nearestPoi(res).name + '附近' }),
      fail: () => this.setData({ place: geo.DEFAULT_CENTER.place + '附近（未获取到定位）' })
    });
  },

  onShow() { this.loadHistory(); },

  setType(no) {
    const type = String(no).charAt(0) === '8' ? 'ebike' : 'bike';
    if (type === this.data.type && this.data.parts.length) return;
    const parts = PARTS[type].map(p => ({ name: p, checked: this.data.picked.indexOf(p) >= 0 }));
    this.setData({ type, parts, picked: this.data.picked.filter(p => PARTS[type].indexOf(p) >= 0) });
  },

  loadHistory() {
    const history = store.faults().map(f => Object.assign({}, f, { statusTx: STATUS[f.status], at: store.fmtTime(f.createdAt), partsTx: f.parts.join('、') }));
    this.setData({ history });
  },

  onNo(e) {
    const no = e.detail.value;
    this.setData({ no, err: '' });
    this.setType(no);
  },

  onParts(e) {
    const picked = e.detail.value;
    this.setData({ picked, err: '', parts: this.data.parts.map(p => ({ name: p.name, checked: picked.indexOf(p.name) >= 0 })) });
  },

  onDesc(e) { this.setData({ desc: e.detail.value }); },

  onPhoto() {
    if (this.data.photos.length >= 3) { wx.showToast({ title: '最多上传 3 张照片', icon: 'none' }); return; }
    wx.chooseMedia({
      count: 3 - this.data.photos.length,
      mediaType: ['image'],
      sourceType: ['camera', 'album'],
      sizeType: ['compressed'],
      success: (res) => this.setData({ photos: this.data.photos.concat(res.tempFiles.map(f => f.tempFilePath)) }),
      fail: (err) => {
        if (/cancel/.test(err.errMsg || '')) return;
        wx.showToast({ title: '暂时无法打开相机，可只填写文字描述提交', icon: 'none' });
      }
    });
  },

  onDelPhoto(e) {
    const photos = this.data.photos.slice();
    photos.splice(Number(e.currentTarget.dataset.i), 1);
    this.setData({ photos });
  },

  onPreview(e) { wx.previewImage({ urls: this.data.photos, current: e.currentTarget.dataset.src }); },

  onSubmit() {
    const no = String(this.data.no).trim();
    if (!/^\d{9}$/.test(no)) { this.setData({ err: '请填写 9 位车辆编号（车头二维码下方）' }); return; }
    if (!this.data.picked.length) { this.setData({ err: '请至少选择一个故障部位' }); return; }
    if (this.data.desc.length > 0 && this.data.desc.trim().length < 4) { this.setData({ err: '描述太短了，再具体一点方便运维定位问题' }); return; }
    const list = store.faults();
    if (list.some(f => f.no === no && f.status !== 'done')) { this.setData({ err: '这辆车已经报修过，运维正在处理' }); return; }
    const now = Date.now();
    const item = {
      id: 'GZ' + String(now).slice(-10), no, type: this.data.type, parts: this.data.picked.slice(),
      desc: this.data.desc.trim(), photos: this.data.photos.length, place: this.data.place,
      createdAt: now, status: 'pending'
    };
    list.unshift(item);
    store.saveFaults(list);
    wx.showToast({ title: '提交成功', icon: 'success' });
    this.setData({ submitted: { id: item.id, no } });
    this.loadHistory();
  },

  onAgain() {
    this.setData({ submitted: null, no: '', picked: [], desc: '', photos: [], err: '', parts: PARTS.bike.map(p => ({ name: p, checked: false })), type: 'bike' });
  },

  // 自己的上报可撤销（比如车其实没坏）
  onRevoke(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '撤销这条上报？', content: '撤销后这辆车可以正常开锁使用', confirmText: '撤销', confirmColor: '#e5483c',
      success: (r) => {
        if (!r.confirm) return;
        store.saveFaults(store.faults().filter(f => f.id !== id));
        this.loadHistory();
      }
    });
  },

  goHome() { wx.reLaunch({ url: '/pages/index/index' }); }
});
