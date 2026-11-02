const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const { STAFF, REVIEW_TAGS } = require('../../utils/data.js');
const MAX_PHOTOS = 3;
const MAX_TEXT = 300;
// 低分必须写清原因，方便客服回访；高分文字选填
function rulesFor(star) {
  return star <= 3
    ? [{ required: true, msg: '打了 ' + star + ' 星，请说说哪里不满意' }, { min: 10, msg: '至少写 10 个字，客服会据此回访' }, { max: MAX_TEXT, msg: '最多 ' + MAX_TEXT + ' 字' }]
    : [{ max: MAX_TEXT, msg: '最多 ' + MAX_TEXT + ' 字' }];
}
Page({
  data: {
    order: null, staff: null, star: 5, word: '超赞', tags: REVIEW_TAGS.map((t) => ({ t, on: false })),
    text: '', photos: [], anonymous: false, rules: rulesFor(5), rateTimes: 0, maxText: MAX_TEXT
  },
  onLoad(q) {
    const order = db.getOrder(q.id);
    if (!order) return;
    this.setData({ order, staff: STAFF.find((s) => s.id === order.staffId) });
  },
  // star-rate 自己的 bind:change
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  onStar(e) { this.setData({ star: e.detail.value, word: e.detail.word, rules: rulesFor(e.detail.value) }); },
  // 同一个事件 bubbles + composed 冒到外层 view 上：统计改了几次分（埋点用）
  onRateBubble() { this.setData({ rateTimes: this.data.rateTimes + 1 }); },
  onTags(e) {
    const on = e.detail.value;
    this.setData({ tags: this.data.tags.map((x) => ({ t: x.t, on: on.indexOf(x.t) >= 0 })) });
  },
  onText(e) { this.setData({ text: e.detail.value }); },
  onAnon(e) { this.setData({ anonymous: e.detail.value }); },
  addPhoto() {
    const left = MAX_PHOTOS - this.data.photos.length;
    if (left <= 0) { wx.showToast({ title: '最多 ' + MAX_PHOTOS + ' 张', icon: 'none' }); return; }
    util.choosePhotos(left, (paths) => this.setData({ photos: this.data.photos.concat(paths).slice(0, MAX_PHOTOS) }));
  },
  useSample() {
    const left = MAX_PHOTOS - this.data.photos.length;
    this.setData({ photos: this.data.photos.concat(['/images/photo/kitchen.jpg', '/images/photo/bathroom.jpg'].slice(0, left)) });
  },
  delPhoto(e) { const i = +e.currentTarget.dataset.i; this.setData({ photos: this.data.photos.filter((x, k) => k !== i) }); },
  preview(e) { wx.previewImage({ current: e.currentTarget.dataset.src, urls: this.data.photos }); },
  submit() {
    if (this.submitting) return;
    // 直接拿 form-item 组件实例调 behavior 里的 validate()
    const fi = this.selectComponent('#rvText');
    const star = this.selectComponent('#star');
    if (fi && !fi.validate(this.data.text)) { wx.showToast({ title: fi.data.error, icon: 'none' }); return; }
    const d = this.data, value = star ? star.data.value : d.star;
    this.submitting = true;
    wx.showLoading({ title: d.photos.length ? '上传照片中' : '提交中', mask: true });
    util.uploadPhotos(d.photos, (photos) => {
      const o = db.getOrder(d.order.id);
      o.review = { star: value, tags: d.tags.filter((x) => x.on).map((x) => x.t), text: d.text.trim(), photos, anonymous: d.anonymous, t: util.stamp() };
      o.status = 'done';
      db.saveOrder(db.pushTimeline(o, '已评价 ' + value + ' 星'));
      wx.hideLoading();
      wx.showToast({ title: '感谢你的评价', icon: 'success' });
      setTimeout(() => wx.navigateBack(), 600);
    });
  }
});
