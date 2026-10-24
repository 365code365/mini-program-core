const S = require('../../utils/store.js');
const { ZONES } = require('../../utils/data.js');

function zoneOf(code) { return ZONES.find(z => z.id === code.charAt(0)); }

// 校验桌号：区号 A/B/C + 两位数字，必须在该区范围内且未被占用
function validate(code) {
  if (!code) return '请输入桌号';
  if (!/^[ABC]\d{2}$/.test(code)) return '格式应为区号+两位数字，如 A05';
  const z = zoneOf(code);
  const n = Number(code.slice(1));
  if (n < 1 || n > z.count) return z.name + '区只有 ' + z.id + '01~' + z.id + S.pad(z.count);
  if (z.occupied.indexOf(n) >= 0) return code + ' 已有顾客就餐，请换一桌';
  return '';
}

Page({
  data: {
    table: '',
    zoneIdx: 0,
    zones: [],
    tables: [],
    input: '',
    error: '',
    selected: '',
    scanning: false,
    canConfirm: false
  },
  onLoad() {
    this.setData({ zones: ZONES.map(z => ({ id: z.id, name: z.name })) });
    this.buildTables(0);
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    this.setData({ table: S.getTable() });
  },
  onUnload() { clearTimeout(this.scanTimer); },
  buildTables(zoneIdx) {
    const z = ZONES[zoneIdx];
    const tables = [];
    for (let i = 1; i <= z.count; i++) {
      const code = z.id + S.pad(i);
      tables.push({ code, busy: z.occupied.indexOf(i) >= 0, seats: z.id === 'C' ? 8 : (i % 3 === 0 ? 6 : 4) });
    }
    this.setData({ zoneIdx, tables });
  },
  onZone(e) { this.buildTables(Number(e.currentTarget.dataset.i)); },
  // 受控输入：转大写、去掉非字母数字、最多 3 位
  onInput(e) {
    const v = (e.detail.value || '').toUpperCase().replace(/[^A-Z0-9]/g, '').slice(0, 3);
    const error = v.length === 3 ? validate(v) : '';
    this.setData({ input: v, error, selected: v.length === 3 && !error ? v : '', canConfirm: v.length === 3 && !error });
    return v;
  },
  onConfirmInput() {
    const error = validate(this.data.input);
    this.setData({ error, canConfirm: !error, selected: error ? '' : this.data.input });
  },
  onPick(e) {
    const code = e.currentTarget.dataset.code;
    // 占用状态从 data 里查，不放进 dataset：引擎会把 data-busy="{{false}}" 变成字符串 "false"（微信里是布尔值）
    const t = this.data.tables.find(x => x.code === code);
    if (t && t.busy) { wx.showToast({ title: code + ' 已占用', icon: 'none' }); return; }
    this.setData({ selected: code, input: code, error: '', canConfirm: true });
  },
  // 模拟扫码：扫描动画 1.5 秒后「识别」出桌贴上的桌号
  onScan() {
    if (this.data.scanning) return;
    this.setData({ scanning: true });
    this.scanTimer = setTimeout(() => {
      const code = 'B06';
      this.setData({ scanning: false, selected: code, input: code, error: '', canConfirm: true, zoneIdx: 1 });
      this.buildTables(1);
      wx.showToast({ title: '识别成功：' + code, icon: 'none' });
    }, 1500);
  },
  cancelScan() { clearTimeout(this.scanTimer); this.setData({ scanning: false }); },
  onSeat() {
    if (!this.data.canConfirm) return;
    S.setTable(this.data.selected);
    this.setData({ table: this.data.selected });
    wx.switchTab({ url: '/pages/menu/menu' });
  },
  goMenu() { wx.switchTab({ url: '/pages/menu/menu' }); }
});
