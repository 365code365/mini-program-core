const S = require('../../utils/store.js');
const host = require('../../utils/host.js');
const { ZONES, DEMO_STICKER } = require('../../utils/data.js');

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

// 从桌贴内容里认出桌号：二维码 https://…/t?table=B06、小程序码 path 里的 table=B06 / scene=B06（scene 可能是编码过的
// table%3DB06），或者桌贴上就是「B06」。别家的二维码认不出来返回 ''
function parseTable(text) {
  let s = String(text || '');
  try { s = decodeURIComponent(s); } catch (e) {}
  const q = /[?&=](?:table|scene)=([A-Za-z]\d{2})(?![0-9])/.exec(s);
  if (q) return q[1].toUpperCase();
  return /^\s*[A-Za-z]\d{2}\s*$/.test(s) ? s.trim().toUpperCase() : '';
}

Page({
  data: {
    table: '',
    zoneIdx: 0,
    zones: [],
    tables: [],
    input: '',
    // 不是用户打字改了桌号（点桌台、扫码）时 +1：输入框换个 id 重建（用户打过字的 input，setData 改 value 画面不跟）
    inputSeq: 0,
    inputFocus: false,
    error: '',
    selected: '',
    scanning: false,
    canConfirm: false
  },
  onLoad(q) {
    this.setData({ zones: ZONES.map(z => ({ id: z.id, name: z.name })) });
    this.buildTables(0);
    // 微信里扫桌贴上的小程序码会直接打开这一页，桌号在 ?table=B06 或 ?scene=B06 里
    const raw = (q && (q.table || q.scene)) || '';
    const code = raw ? parseTable('?table=' + raw) : '';
    if (code) this.applyCode(code, '已识别桌号');
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    this.setData({ table: S.getTable() });
  },
  onHide() { this.cancelScan(); },
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
  onInputBlur() { this.setData({ inputFocus: false }); },
  onConfirmInput() {
    const error = validate(this.data.input);
    this.setData({ error, canConfirm: !error, selected: error ? '' : this.data.input });
  },
  onPick(e) {
    const code = String(e.currentTarget.dataset.code);
    // 占用状态从 data 里查，不放进 dataset：引擎会把 data-busy="{{false}}" 变成字符串 "false"（微信里是布尔值）
    const t = this.data.tables.find(x => x.code === code);
    if (t && t.busy) { wx.showToast({ title: code + ' 已占用', icon: 'none' }); return; }
    this.setData({ selected: code, input: code, inputSeq: this.data.inputSeq + 1, error: '', canConfirm: true });
  },
  // 扫码 / 带参打开认出来的桌号：校验、切到那一区、填进输入框
  applyCode(code, how) {
    const error = validate(code);
    if (error) {
      wx.showToast({ title: error, icon: 'none' });
      return false;
    }
    this.buildTables(ZONES.findIndex(z => z.id === code.charAt(0)));
    this.setData({ selected: code, input: code, inputSeq: this.data.inputSeq + 1, error: '', canConfirm: true });
    wx.showToast({ title: how + '：' + code, icon: 'none' });
    return true;
  },
  // 先调真扫码（微信、接了 hostApis 的宿主）；宿主没有扫码能力就退回页内取景框；用户取消什么都不做
  onScan() {
    if (this.data.scanning || this.asking) return;
    this.asking = true;
    host.scan().then(r => {
      this.asking = false;
      if (r.cancel) return;
      if (r.unsupported) {
        this.demoScan();
        return;
      }
      this.useScanned(r.text);
    });
  },
  useScanned(text) {
    const code = parseTable(text);
    if (!code) {
      wx.showModal({
        title: '没认出桌号',
        content: '这不是禾味食堂的桌贴二维码。请扫桌角的二维码，或手动输入桌贴下方印的桌号',
        showCancel: false,
        confirmText: '知道了'
      });
      return;
    }
    this.applyCode(code, '识别成功');
  },
  // 页内取景框：1.5 秒后识别演示桌贴（内容与真桌贴同一格式，走同一套解析与校验）；随时可以改成手动输入
  demoScan() {
    this.setData({ scanning: true });
    this.scanTimer = setTimeout(() => {
      this.setData({ scanning: false });
      this.useScanned(DEMO_STICKER);
    }, 1500);
  },
  cancelScan() {
    clearTimeout(this.scanTimer);
    if (this.data.scanning) this.setData({ scanning: false });
  },
  manualInput() {
    clearTimeout(this.scanTimer);
    this.setData({ scanning: false, inputFocus: true });
  },
  onSeat() {
    if (!this.data.canConfirm) {
      wx.showToast({ title: '先扫码、输入或选择一张空闲桌', icon: 'none' });
      return;
    }
    S.setTable(this.data.selected);
    this.setData({ table: this.data.selected });
    wx.switchTab({ url: '/pages/menu/menu' });
  },
  goMenu() { wx.switchTab({ url: '/pages/menu/menu' }); }
});
