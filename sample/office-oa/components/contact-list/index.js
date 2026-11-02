// 通讯录列表：搜索 + 部门筛选 + A-Z 分组 + 右侧索引栏（按住滑动跳到对应字母，中间显示大字母）。
// 索引栏的命中不靠 selectorQuery：栏的位置与每格高度都是这里按窗口尺寸算出来、再写进 style 的，
// 所以 touches[0].clientY 直接换算成第几个字母，滑动时不用等异步回包。
const O = require('../../utils/org.js');

const ITEM_RPX = 36;

Component({
  properties: {
    pick: { type: Boolean, value: false },
    exclude: { type: String, value: '' }
  },
  data: {
    depts: [{ id: '', name: '全部' }].concat(O.depts), dept: '', kw: '',
    groups: [], letters: [], total: 0, toView: '', hint: '', barTop: 0, itemH: 18
  },
  lifetimes: {
    attached() {
      this.layoutBar();
      this.build();
      const app = getApp();
      this.offResize = app.onResize ? app.onResize(() => this.layoutBar()) : null;
    },
    detached() { this.offResize && this.offResize(); clearTimeout(this.hideT); }
  },
  methods: {
    layoutBar() {
      const win = wx.getWindowInfo ? wx.getWindowInfo() : wx.getSystemInfoSync();
      const itemH = ITEM_RPX * win.windowWidth / 750;
      this.win = win;
      this.setData({ itemH });
      this.placeBar();
    },
    placeBar() {
      if (!this.win) return;
      const barH = this.data.letters.length * this.data.itemH;
      this.setData({ barTop: Math.max(60, Math.round((this.win.windowHeight - barH) / 2)) });
    },
    build() {
      const { dept, kw } = this.data;
      const k = kw.trim().toLowerCase();
      const list = O.employees.filter(e => e.id !== this.data.exclude && (!dept || e.dept === dept) &&
        (!k || e.name.indexOf(k) >= 0 || e.py.indexOf(k) >= 0 || e.title.toLowerCase().indexOf(k) >= 0 || e.phone.indexOf(k) >= 0 || e.deptName.indexOf(k) >= 0));
      list.sort((a, b) => (a.py < b.py ? -1 : 1));
      const groups = [];
      list.forEach(e => {
        let g = groups[groups.length - 1];
        if (!g || g.letter !== e.letter) { g = { letter: e.letter, items: [] }; groups.push(g); }
        g.items.push(e);
      });
      this.setData({ groups, letters: groups.map(g => g.letter), total: list.length });
      this.placeBar();
    },
    onKw(e) { this.setData({ kw: e.detail.value }); this.build(); },
    clearKw() { this.setData({ kw: '' }); this.build(); },
    pickDept(e) { this.setData({ dept: e.currentTarget.dataset.id }); this.build(); },
    // ---------- 索引栏 ----------
    letterAt(e) {
      const t = (e.touches && e.touches[0]) || (e.changedTouches && e.changedTouches[0]);
      if (!t) return -1;
      const i = Math.floor((t.clientY - this.data.barTop) / this.data.itemH);
      return Math.max(0, Math.min(this.data.letters.length - 1, i));
    },
    jump(i) {
      const L = this.data.letters[i];
      if (!L || L === this.data.hint) return;
      this.setData({ hint: L, toView: 'L-' + L });
    },
    onBarStart(e) { clearTimeout(this.hideT); this.jump(this.letterAt(e)); },
    onBarMove(e) { this.jump(this.letterAt(e)); },
    onBarEnd() {
      clearTimeout(this.hideT);
      this.hideT = setTimeout(() => this.setData({ hint: '' }), 400);
    },
    tapItem(e) {
      const emp = O.emp(e.currentTarget.dataset.id);
      if (this.data.pick) this.triggerEvent('select', emp);
      else wx.navigateTo({ url: '/pages/employee/employee?id=' + emp.id });
    }
  }
});
