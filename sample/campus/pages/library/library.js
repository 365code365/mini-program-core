// 图书馆：馆藏检索（书名 / 作者 / 索书号）、借阅（每人最多 10 本，借期 30 天）、续借（一次，+30 天）、复制索书号
const D = require('../../utils/data.js');
const store = require('../../utils/store.js');
const kit = require('../../utils/kit.js');
Page({
  data: { kw: '', results: D.BOOKS, borrows: [], tab: 'search' },
  onShow() { this.loadBorrows(); },
  loadBorrows() {
    const today = kit.dateStr(0);
    const borrows = store.borrows().map(b => {
      const left = Math.round((new Date(b.due.replace(/-/g, '/')) - new Date(today.replace(/-/g, '/'))) / 86400000);
      return Object.assign({}, b, { left, overdue: left < 0 });
    });
    this.setData({ borrows });
  },
  setTab(e) { this.setData({ tab: e.currentTarget.dataset.t }); },
  onKw(e) { this.setData({ kw: e.detail.value }); this.search(); },
  search() {
    const k = this.data.kw.trim().toLowerCase();
    this.setData({ results: !k ? D.BOOKS : D.BOOKS.filter(b => (b.title + b.author + b.call).toLowerCase().indexOf(k) >= 0) });
  },
  copyCall(e) { wx.setClipboardData({ data: e.currentTarget.dataset.call, success: () => wx.showToast({ title: '索书号已复制，到书架按号找书', icon: 'none' }) }); },
  borrow(e) {
    const b = D.BOOKS.find(x => x.id === e.currentTarget.dataset.id);
    const list = store.borrows();
    if (list.some(x => x.bookId === b.id)) { kit.toast('这本书你已经借了'); return; }
    if (list.length >= 10) { kit.toast('最多同时借阅 10 本'); return; }
    if (!b.left) { kit.toast('已全部借出，可预约到馆通知'); return; }
    wx.showModal({
      title: '预约借书', content: '《' + b.title + '》\n馆藏位置：' + b.loc + '\n预约后请在 2 天内凭校园卡到总服务台取书。', confirmText: '预约', confirmColor: '#14937f',
      success: r => {
        if (!r.confirm) return;
        list.push({ id: kit.uid('r'), bookId: b.id, title: b.title, call: b.call, borrowed: kit.dateStr(0), due: kit.dateStr(30), renewed: false });
        store.saveBorrows(list);
        this.loadBorrows();
        kit.toast('预约成功，应还日期 ' + kit.dateStr(30));
      }
    });
  },
  renew(e) {
    const list = store.borrows(), b = list.find(x => x.id === e.currentTarget.dataset.id);
    if (b.renewed) { kit.toast('每本书只能续借一次'); return; }
    const d = new Date(b.due.replace(/-/g, '/'));
    b.due = kit.fmt(d.getTime() + 30 * 86400000, 'YYYY-MM-DD');
    b.renewed = true;
    store.saveBorrows(list);
    this.loadBorrows();
    kit.toast('续借成功，新的应还日期 ' + b.due);
  },
  giveBack(e) {
    const id = e.currentTarget.dataset.id;
    kit.confirm('归还图书', '请将图书放入任意自助还书机，确认已归还？').then(ok => {
      if (!ok) return;
      store.saveBorrows(store.borrows().filter(x => x.id !== id));
      this.loadBorrows();
    });
  },
  preview(e) { wx.previewImage({ current: e.currentTarget.dataset.src, urls: D.BOOKS.map(b => b.cover) }); }
});
