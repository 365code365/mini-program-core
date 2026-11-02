const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const { SERVICES, STAFF } = require('../../utils/data.js');

const MAX_PHOTOS = 4;

Page({
  data: {
    svc: null, specIdx: 0, qty: 1, addr: null, addrText: '',
    date: '', minDate: '', maxDate: '', slots: util.timeSlots(), slotIdx: -1,
    coupon: null, coupons: [], remark: '', photos: [], amount: 0, pay: 0, errors: {}, staffName: ''
  },
  onLoad(q) {
    const svc = SERVICES.find((s) => s.id === q.id) || SERVICES[0];
    const specIdx = Math.max(0, svc.specs.findIndex((x) => x.id === q.spec));
    this.preferStaff = q.staff || '';
    const staff = (q.staff && STAFF.find((s) => s.id === q.staff)) || db.staffFor(svc.id);
    this.setData({
      svc, specIdx, qty: svc.minQty, staffName: staff.name,
      date: util.addDays(1), minDate: util.addDays(0), maxDate: util.addDays(14)
    });
    this.setAddr(db.defaultAddr());
    this.calc();
  },
  onShow() {
    // 从地址页选完 / 新增完回来
    const picked = db.takePicked();
    if (picked) this.setAddr(db.getAddr(picked));
    else if (this.data.addr && !db.getAddr(this.data.addr.id)) this.setAddr(db.defaultAddr());
  },
  setAddr(a) { this.setData({ addr: a, addrText: db.fullAddr(a), 'errors.addr': '' }); },
  calc() {
    const d = this.data, price = d.svc.specs[d.specIdx].price, amount = price * d.qty;
    const coupons = db.usableCoupons(d.svc.id, amount);
    // 金额变了导致当前券不满足门槛：自动换成可用的最大面额
    let coupon = d.coupon && coupons.find((c) => c.id === d.coupon.id);
    if (d.coupon === null && this.autoCoupon !== false) coupon = coupons.slice().sort((a, b) => b.amount - a.amount)[0];
    this.setData({ amount, coupons, coupon: coupon || null, pay: Math.max(0, amount - (coupon ? coupon.amount : 0)) });
  },
  onSpec(e) { this.setData({ specIdx: +e.detail.value }); this.calc(); },
  onQty(e) { this.setData({ qty: e.detail.value }); this.calc(); },
  goAddr() { wx.navigateTo({ url: '/pages/address/address?select=1' + (this.data.addr ? '&cur=' + this.data.addr.id : '') }); },
  onDate(e) { this.setData({ date: e.detail.value, 'errors.time': '' }); },
  onSlot(e) { this.setData({ slotIdx: +e.detail.value, 'errors.time': '' }); },
  pickCoupon() {
    const list = this.data.coupons;
    if (!list.length) { wx.showToast({ title: '暂无满足门槛的优惠券', icon: 'none' }); return; }
    wx.showActionSheet({
      itemList: list.map((c) => c.name + ' 减 ¥' + c.amount).concat(['不使用优惠券']),
      success: (r) => {
        this.autoCoupon = false;
        this.setData({ coupon: r.tapIndex < list.length ? list[r.tapIndex] : null });
        this.calc();
      }
    });
  },
  onRemark(e) { this.setData({ remark: e.detail.value }); },
  addPhoto() {
    const left = MAX_PHOTOS - this.data.photos.length;
    if (left <= 0) { wx.showToast({ title: '最多 ' + MAX_PHOTOS + ' 张', icon: 'none' }); return; }
    util.choosePhotos(left, (paths) => this.setData({ photos: this.data.photos.concat(paths).slice(0, MAX_PHOTOS) }));
  },
  useSample() {
    const left = MAX_PHOTOS - this.data.photos.length;
    if (left <= 0) return;
    this.setData({ photos: this.data.photos.concat(['/images/photo/kitchen.jpg', '/images/photo/bathroom.jpg'].slice(0, left)) });
  },
  photoOp(e) {
    const i = +e.currentTarget.dataset.i;
    wx.showActionSheet({
      itemList: ['查看大图', '删除'],
      success: (r) => {
        if (r.tapIndex === 0) wx.previewImage({ current: this.data.photos[i], urls: this.data.photos });
        else this.setData({ photos: this.data.photos.filter((x, k) => k !== i) });
      }
    });
  },
  validate() {
    const d = this.data, errors = {};
    if (!d.addr) errors.addr = '请选择服务地址';
    if (d.slotIdx < 0) errors.time = '请选择上门时段';
    else if (d.date === d.minDate) {
      // 当天的单至少提前 2 小时
      const p = d.slots[d.slotIdx].split(':'), now = new Date();
      if (+p[0] * 60 + +p[1] < now.getHours() * 60 + now.getMinutes() + 120) errors.time = '当天预约请至少提前 2 小时';
    }
    this.setData({ errors });
    return !Object.keys(errors).length;
  },
  submit() {
    if (!this.validate()) { wx.showToast({ title: this.data.errors.addr || this.data.errors.time, icon: 'none' }); return; }
    const d = this.data, spec = d.svc.specs[d.specIdx], now = new Date();
    wx.showLoading({ title: d.photos.length ? '上传照片中' : '提交中', mask: true });
    util.uploadPhotos(d.photos, (photos) => {
      const staff = (this.preferStaff && STAFF.find((s) => s.id === this.preferStaff)) || db.staffFor(d.svc.id);
      const id = 'HG' + util.ymd(now).replace(/-/g, '') + util.pad(now.getHours()) + util.pad(now.getMinutes()) + util.pad(now.getSeconds());
      const order = {
        id, svcId: d.svc.id, svcName: d.svc.name, cover: d.svc.cover, specName: spec.name, qty: d.qty,
        unit: d.svc.type === 'hourly' ? '小时' : d.svc.unit, price: spec.price, amount: d.amount,
        discount: d.coupon ? d.coupon.amount : 0, couponId: d.coupon ? d.coupon.id : '', pay: d.pay,
        addr: { name: d.addr.name, phone: d.addr.phone, full: d.addrText }, date: d.date, time: d.slots[d.slotIdx],
        remark: d.remark.trim(), photos, status: 'waiting', staffId: staff.id, createdAt: util.stamp(now),
        cancelReason: '', invoiced: false, review: null,
        timeline: [{ t: util.stamp(now), text: '下单成功，已为你安排' + staff.name + '师傅' }]
      };
      db.saveOrder(order);
      if (d.coupon) db.setCouponStatus(d.coupon.id, 'used');
      wx.hideLoading();
      wx.redirectTo({ url: '/pages/order-result/order-result?id=' + id });
    });
  }
});
