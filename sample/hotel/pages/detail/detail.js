const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
// 点评标签的占比（示例数据：按总点评数折算）
const TAG_SHARE = [0.12, 0.09, 0.15, 0.06, 0.05, 0.08];
Page({
  data: { h: null, cur: 0, fav: false, rooms: [], hourRooms: [], per: 2, reviews: [], rvCount: 0, tagCounts: [], hourly: false },
  onLoad(q) {
    const h = data.hotelOf(q.id);
    this.h = h;
    this.setData({ h, fav: store.isFav(h.id) });
    wx.setNavigationBarTitle({ title: h.name.length > 12 ? h.name.slice(0, 12) + '…' : h.name });
    store.addView(h.id);
  },
  onShow() {
    this.loadDates();
    this.loadReviews();
  },
  loadDates() {
    const s = store.getSearch();
    const h = this.h;
    const per = Math.ceil((s.adults + s.children) / s.rooms);
    const t = store.today();
    this.setData({
      s, per, hourly: s.kind === 2, today: t, maxDate: store.addDays(t, 180), minOut: store.addDays(s.checkin, 1),
      inShort: store.short(s.checkin), inWeek: store.week(s.checkin), outShort: store.short(s.checkout), outWeek: store.week(s.checkout),
      nights: store.nights(s.checkin, s.checkout), people: store.peopleText(s),
      rooms: h.rooms.filter(r => !r.hours).map(r => Object.assign({}, r, { fit: r.max >= per })),
      hourRooms: h.rooms.filter(r => r.hours)
    });
  },
  loadReviews() {
    const h = this.h;
    const all = store.reviewsOf(h);
    const mine = all.filter(r => r.mine).length;
    this.setData({
      reviews: all.slice(0, 2), rvCount: h.reviews + mine,
      tagCounts: data.RATE_TAGS.map((t, i) => ({ t, n: Math.round(h.reviews * TAG_SHARE[i]) }))
    });
  },
  onSwiper(e) { this.setData({ cur: e.detail.current }); },
  onPreview() { wx.previewImage({ current: this.h.images[this.data.cur], urls: this.h.images }); },
  onFav() {
    const fav = store.toggleFav(this.h.id);
    this.setData({ fav });
    wx.showToast({ title: fav ? '已收藏' : '已取消收藏', icon: fav ? 'success' : 'none' });
  },
  onCall() {
    const h = this.h;
    wx.showActionSheet({
      itemList: ['拨打酒店电话 ' + h.phone, '联系途栖客服 ' + data.SERVICE_PHONE],
      success: r => {
        if (r.tapIndex === 0) host.call(h.phone, h.name);
        else host.call(data.SERVICE_PHONE, '途栖客服');
      }
    });
  },
  onMap() { host.openMap(this.h); },
  // 日期直接在详情页改，回到列表 / 预订页都按新的日期
  onCheckin(e) {
    const s = store.getSearch();
    s.checkin = e.detail.value;
    if (s.checkout <= s.checkin) s.checkout = store.addDays(s.checkin, 1);
    store.saveSearch(s);
    this.loadDates();
  },
  onCheckout(e) {
    const s = store.getSearch();
    s.checkout = e.detail.value;
    store.saveSearch(s);
    this.loadDates();
  },
  onBook(e) {
    const rid = String(e.currentTarget.dataset.rid);
    const room = this.h.rooms.find(r => r.id === rid) || this.h.rooms[0];
    const go = () => wx.navigateTo({ url: '/pages/booking/booking?hid=' + this.h.id + '&rid=' + room.id });
    if (!room.hours && room.max < this.data.per) {
      wx.showModal({
        title: '人数超出房型可住人数', content: '「' + room.name + '」每间最多住 ' + room.max + ' 人，当前 ' + this.data.people + '，预订时会按人数多订房间。',
        confirmText: '继续预订', success: r => { if (r.confirm) go(); }
      });
      return;
    }
    go();
  },
  onBookFirst() {
    const r = this.data.hourly && this.data.hourRooms.length ? this.data.hourRooms[0] : (this.data.rooms.find(x => x.fit) || this.data.rooms[0]);
    this.onBook({ currentTarget: { dataset: { rid: r.id } } });
  },
  onReviews(e) {
    const tag = e.currentTarget.dataset.tag;
    wx.navigateTo({ url: '/pages/reviews/reviews?hid=' + this.h.id + (tag ? '&tag=' + encodeURIComponent(tag) : '') });
  }
});
