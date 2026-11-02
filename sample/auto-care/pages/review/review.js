const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const { REVIEW_TAGS } = require('../../utils/data.js');

const STAR_WORD = ['', '很差', '较差', '一般', '满意', '非常满意'];

Page({
  data: { order: null, star: 5, word: STAR_WORD[5], stars: [1, 2, 3, 4, 5], tags: [], text: '', err: '' },
  onLoad(q) {
    const order = db.getOrder(q.id);
    this.setData({ order, tags: REVIEW_TAGS.map((t) => ({ name: t, on: false })) });
  },
  onStar(e) { const n = +e.currentTarget.dataset.n; this.setData({ star: n, word: STAR_WORD[n], err: '' }); },
  onTag(e) { const i = e.currentTarget.dataset.i; this.setData({ ['tags[' + i + '].on']: !this.data.tags[i].on }); },
  onText(e) { this.setData({ text: e.detail.value, err: '' }); },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  submit() {
    const o = this.data.order;
    if (!o || o.status !== 'done') { wx.showToast({ title: '这个订单不能评价', icon: 'none' }); return; }
    const tags = this.data.tags.filter((t) => t.on).map((t) => t.name);
    // 差评必须写原因，方便门店改进
    if (this.data.star <= 3 && this.data.text.trim().length < 5) { this.setData({ err: '打 3 星及以下请写下至少 5 个字的原因' }); return; }
    const review = { star: this.data.star, starText: '★★★★★'.slice(0, this.data.star) + '☆☆☆☆☆'.slice(0, 5 - this.data.star) + ' ' + this.data.word,
      tags, text: this.data.text.trim(), t: util.stamp() };
    o.review = review; o.status = 'reviewed';
    o.timeline = o.timeline.concat([{ t: review.t, text: '已评价 ' + this.data.star + ' 星' }]);
    db.saveOrder(o);
    wx.showToast({ title: '评价成功', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 600);
  }
});
