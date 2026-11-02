const D = require('../../utils/data.js');
const store = require('../../utils/store.js');

const PHOTOS = ['/images/rooms/r1-raw.jpg', '/images/rooms/r3-raw.jpg', '/images/rooms/r6-raw.jpg', '/images/rooms/r2.jpg'];
// 每个阶段业主要确认的事
const CONFIRM = ['设计师已确认上门时间', '已完成上门量房', '已确认设计方案', '已签订施工合同', '已完成开工交底', '水电验收合格', '泥木验收合格', '油漆验收合格', '安装全部完成', '竣工验收通过'];

Page({
  data: { p: null, isNew: false, stages: [], percent: 0, designer: null, diaries: [], text: '', photo: '', err: '', done: false },

  onLoad(q) {
    this._id = q.id;
    this.setData({ isNew: q.new === '1' });
  },
  onShow() { this.load(); },

  load() {
    const p = this._id ? store.project(this._id) : store.projects()[0];
    if (!p) { this.setData({ p: null }); return; }
    this._id = p.id;
    const last = store.STAGES.length - 1;
    const stages = store.STAGES.map((name, i) => {
      const log = p.logs.filter((l) => l.stage === i).pop();
      return { i, name, state: i < p.stage ? 'done' : (i === p.stage ? 'cur' : 'todo'), time: log ? store.ymd(log.ts) : '' };
    });
    const diaries = (p.diaries || []).slice().sort((a, b) => b.ts - a.ts).map((y) => Object.assign({}, y, { time: store.ymdhm(y.ts), stageName: store.STAGES[y.stage] }));
    this.setData({
      p, stages, diaries, percent: store.percent(p), done: p.stage >= last && !!p.finished,
      designer: p.designer ? D.designer(p.designer) : null, next: CONFIRM[p.stage]
    });
    wx.setNavigationBarTitle({ title: p.stage === 0 ? '预约详情' : '装修进度' });
  },

  // 业主确认当前阶段完成 → 进入下一阶段
  advance() {
    const p = this.data.p, last = store.STAGES.length - 1;
    wx.showModal({
      title: '确认「' + store.STAGES[p.stage] + '」完成？',
      content: CONFIRM[p.stage] + '。确认后进入下一阶段' + (p.stage < last ? '「' + store.STAGES[p.stage + 1] + '」' : '，项目完工'),
      success: (r) => {
        if (!r.confirm) return;
        const now = Date.now();
        p.logs.push({ stage: p.stage, ts: now, text: CONFIRM[p.stage] });
        if (p.stage < last) {
          p.stage += 1;
          p.logs.push({ stage: p.stage, ts: now, text: store.STAGES[p.stage] + ' 开始' });
        } else {
          p.finished = true;
        }
        store.saveProject(p);
        wx.showToast({ title: p.finished ? '恭喜完工！' : '已进入' + store.STAGES[p.stage], icon: 'success' });
        this.load();
      }
    });
  },

  cancel() {
    wx.showModal({
      title: '取消这次预约？', content: '取消后设计师不会再上门，可以随时重新预约', confirmText: '取消预约', confirmColor: '#d9534f', cancelText: '再想想',
      success: (r) => {
        if (!r.confirm) return;
        store.set(store.K.projects, store.projects().filter((x) => x.id !== this._id));
        wx.showToast({ title: '已取消', icon: 'none' });
        setTimeout(() => wx.switchTab({ url: '/pages/diary/diary' }), 500);
      }
    });
  },

  call() {
    wx.makePhoneCall({ phoneNumber: '4008261688', fail: () => wx.showToast({ title: '请拨打 400-826-1688', icon: 'none' }) });
  },

  // ---------- 写日记 ----------
  onText(e) { this.setData({ text: e.detail.value, err: '' }); },
  addPhoto() {
    wx.chooseImage({
      count: 1, sizeType: ['compressed'], sourceType: ['album', 'camera'],
      success: (res) => this.setData({ photo: res.tempFilePaths[0] }),
      fail: () => {
        // 桌面 / 拒绝授权时：从工地示例照片里选
        wx.showActionSheet({
          itemList: ['工地照片：客厅', '工地照片：餐厅', '工地照片：卧室', '完工照片：客厅'],
          success: (r) => this.setData({ photo: PHOTOS[r.tapIndex] })
        });
      }
    });
  },
  dropPhoto() { this.setData({ photo: '' }); },
  addDiary() {
    const text = this.data.text.trim();
    if (text.length < 5) return this.setData({ err: '多写几句吧，至少 5 个字' });
    const p = this.data.p;
    p.diaries = [{ id: 'y' + Date.now().toString(36), ts: Date.now(), stage: p.stage, text, photo: this.data.photo }].concat(p.diaries || []);
    store.saveProject(p);
    this.setData({ text: '', photo: '' });
    wx.showToast({ title: '日记已发布', icon: 'success' });
    this.load();
  },
  preview(e) { wx.previewImage({ urls: [e.currentTarget.dataset.src] }); },
  goDesigner() { wx.navigateTo({ url: '/pages/designer/designer?id=' + this.data.designer.id }); }
});
