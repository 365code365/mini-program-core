// 发布闲置：图片（wx.chooseMedia，桌面没有相册时降级用示例图）、标题、描述、成色、价格、分类、包邮。
// 离开页面自动存草稿，下次进来恢复
const store = require('../../utils/store.js');
const D = require('../../utils/data.js');
const SAMPLES = ['g_kindle', 'g_headphone', 'g_camera', 'g_guitar', 'g_yoga', 'g_band'].map(n => '/images/goods/' + n + '.jpg');
const EMPTY = { imgs: [], title: '', desc: '', cond: '', price: '', orig: '', catIdx: -1, free: false, err: {} };
Page({
  data: Object.assign({ conds: D.CONDS, catNames: D.CATS.map(c => c.name), draft: false }, EMPTY),
  onLoad() {
    const d = store.get('xz_draft', null);
    if (d && (d.title || d.imgs.length)) this.setData(Object.assign({}, d, { draft: true, err: {} }));
  },
  onShow() { store.syncTab(this, 1); },
  onHide() { this.saveDraft(); },
  saveDraft() {
    const { imgs, title, desc, cond, price, orig, catIdx, free } = this.data;
    store.set('xz_draft', { imgs, title, desc, cond, price, orig, catIdx, free });
  },
  chooseImg() {
    const left = 9 - this.data.imgs.length;
    if (left <= 0) { wx.showToast({ title: '最多 9 张图片', icon: 'none' }); return; }
    wx.chooseMedia({
      count: left, mediaType: ['image'], sourceType: ['album', 'camera'], sizeType: ['compressed'],
      success: r => this.addImgs(r.tempFiles.map(f => f.tempFilePath)),
      fail: err => {
        if (/cancel/.test(err.errMsg || '')) return;
        wx.showModal({
          title: '无法打开相册', content: '当前设备不能选择图片，先用示例图片继续发布？', confirmText: '用示例图',
          success: m => { if (m.confirm) this.addImgs([SAMPLES[this.data.imgs.length % SAMPLES.length]]); }
        });
      }
    });
  },
  addImgs(list) { this.setData({ imgs: this.data.imgs.concat(list).slice(0, 9), 'err.imgs': '' }); },
  removeImg(e) {
    const imgs = this.data.imgs.slice();
    imgs.splice(e.currentTarget.dataset.i, 1);
    this.setData({ imgs });
  },
  previewImg(e) { wx.previewImage({ urls: this.data.imgs, current: this.data.imgs[e.currentTarget.dataset.i] }); },
  onTitle(e) { this.setData({ title: e.detail.value, 'err.title': '' }); },
  onDesc(e) { this.setData({ desc: e.detail.value }); },
  onPrice(e) { this.setData({ price: e.detail.value, 'err.price': '' }); },
  onOrig(e) { this.setData({ orig: e.detail.value }); },
  pickCond(e) { this.setData({ cond: e.currentTarget.dataset.c, 'err.cond': '' }); },
  onCat(e) { this.setData({ catIdx: Number(e.detail.value), 'err.cat': '' }); },
  onFree(e) { this.setData({ free: e.detail.value }); },
  submit() {
    const d = this.data, err = {};
    const price = Number(d.price), orig = Number(d.orig || 0);
    if (!d.imgs.length) err.imgs = '至少上传 1 张实拍图';
    if (d.title.trim().length < 4) err.title = '标题至少 4 个字，写清品牌型号';
    if (!(price > 0) || price > 99999) err.price = '请填写 1~99999 之间的价格';
    else if (orig && orig < price) err.price = '售价不能高于原价';
    if (!d.cond) err.cond = '请选择成色';
    if (d.catIdx < 0) err.cat = '请选择分类';
    const keys = Object.keys(err);
    if (keys.length) { this.setData({ err }); wx.showToast({ title: err[keys[0]], icon: 'none' }); return; }
    const item = {
      id: 'm' + Date.now(), title: d.title.trim(), desc: d.desc.trim() || '卖家很懒，什么都没写～', price, orig: orig || Math.round(price * 1.6),
      cond: d.cond, cat: D.CATS[d.catIdx].key, imgs: d.imgs, ratio: 1.0, free: d.free, status: 'on', created: Date.now(), polished: '', wants: 0, views: 0
    };
    const list = store.mine();
    list.unshift(item);
    store.saveMine(list);
    store.set('xz_feed_dirty', true);
    store.set('xz_draft', null);
    this.setData(Object.assign({ draft: false }, EMPTY, { err: {} }));
    wx.showToast({ title: '发布成功', icon: 'success' });
    setTimeout(() => wx.navigateTo({ url: '/pages/my-items/my-items' }), 600);
  },
  clearDraft() {
    store.set('xz_draft', null);
    this.setData(Object.assign({ draft: false }, EMPTY, { err: {} }));
  }
});
