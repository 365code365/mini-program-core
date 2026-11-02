const data = require('../../utils/data.js');
const R = require('../../utils/repair.js');
const SAMPLE = '/images/repair-sample.jpg';
const MAX_PHOTOS = 3;
// 故障报修：电站页点「维护中」的终端、我的页「故障报修」进来；带 no=<终端号> 时预填。
// 提交后进「我的报修」看进度。拍照要宿主：桌面窗体 chooseImage 回 fail，常驻「使用示例图」入口
Page({
  data: {
    tab: 'form', seq: 0, no: '', stationName: '', noErr: '', types: R.TYPES, type: '', desc: '', phone: '',
    photos: [], err: '', records: [], pending: 0
  },
  onLoad(q) {
    if (q.tab === 'list') this.setData({ tab: 'list' });
    if (q.no) this.applyNo(decodeURIComponent(q.no));
  },
  onShow() { this.loadRecords(); },
  loadRecords() { this.setData({ records: R.all(), pending: R.pending() }); },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.k }); if (e.currentTarget.dataset.k === 'list') this.loadRecords(); },
  applyNo(v) {
    const no = String(v || '').trim().toUpperCase();
    const hit = no ? data.findTerminal(no) : null;
    this.setData({ no, stationName: hit ? hit.station.name + ' · ' + hit.terminal.power + 'kW' : '', noErr: no && !hit ? '没有这个终端编号，请核对桩身上的编号（如 XX0128-02）' : '' });
  },
  onNo(e) { this.applyNo(e.detail.value); },
  onType(e) { this.setData({ type: e.currentTarget.dataset.t, err: '' }); },
  onDesc(e) { this.setData({ desc: e.detail.value, err: '' }); },
  onPhone(e) { this.setData({ phone: e.detail.value.replace(/\D/g, ''), err: '' }); },
  onAddPhoto() {
    const left = MAX_PHOTOS - this.data.photos.length;
    if (left <= 0) { wx.showToast({ title: '最多 ' + MAX_PHOTOS + ' 张', icon: 'none' }); return; }
    wx.chooseImage({
      count: left, sizeType: ['compressed'], sourceType: ['camera', 'album'],
      success: r => this.setData({ photos: this.data.photos.concat(r.tempFilePaths).slice(0, MAX_PHOTOS) }),
      fail: () => wx.showToast({ title: '没有选到照片，可点「使用示例图」', icon: 'none' })
    });
  },
  onSample() {
    if (this.data.photos.length >= MAX_PHOTOS) { wx.showToast({ title: '最多 ' + MAX_PHOTOS + ' 张', icon: 'none' }); return; }
    this.setData({ photos: this.data.photos.concat(SAMPLE) });
  },
  onPreview(e) { wx.previewImage({ urls: this.data.photos, current: this.data.photos[Number(e.currentTarget.dataset.i)] }); },
  onDelPhoto(e) {
    const photos = this.data.photos.slice();
    photos.splice(Number(e.currentTarget.dataset.i), 1);
    this.setData({ photos });
  },
  validate() {
    const d = this.data;
    if (!d.no) return '请填写终端编号';
    if (d.noErr) return d.noErr;
    if (!d.type) return '请选择问题类型';
    if (d.type === '其他问题' && d.desc.trim().length < 5) return '选「其他问题」时请至少写 5 个字说明情况';
    if (d.phone && !/^1[3-9]\d{9}$/.test(d.phone)) return '手机号格式不对，留空则使用账号手机';
    return '';
  },
  onSubmit() {
    const err = this.validate();
    if (err) { this.setData({ err }); return; }
    const d = this.data;
    const dup = R.duplicate(d.no, d.type);
    if (dup) {
      wx.showModal({ title: '已有报修单', content: d.no + '「' + d.type + '」已于 ' + dup.time + ' 报修，当前进度：' + dup.stepText + '。', confirmText: '查看进度', cancelText: '关闭',
        success: r => { if (r.confirm) this.setData({ tab: 'list' }); } });
      return;
    }
    const hit = data.findTerminal(d.no);
    R.add({ no: d.no, station: hit.station.name, type: d.type, desc: d.desc.trim(), phone: d.phone || '138****6620', photos: d.photos });
    wx.showToast({ title: '已提交，运维会尽快处理', icon: 'none' });
    this.setData({ tab: 'list', seq: d.seq + 1, type: '', desc: '', photos: [], err: '' });
    this.loadRecords();
  },
  onRecord(e) {
    const r = this.data.records[Number(e.currentTarget.dataset.i)];
    wx.showModal({
      title: r.no + ' · ' + r.stepText,
      content: r.station + '\n问题：' + r.type + (r.desc ? '（' + r.desc + '）' : '') + '\n' + r.note + '\n联系电话 ' + r.phone,
      showCancel: false, confirmText: '知道了', confirmColor: '#00B87A'
    });
  }
});
