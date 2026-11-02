const S = require('../../utils/store.js');
const F = require('../../utils/forms.js');
const O = require('../../utils/org.js');
const ask = require('../../utils/ask.js');

const NODE_TEXT = { done: '已同意', pending: '审批中', wait: '待审批', rejected: '已驳回', cancel: '已撤回' };

Page({
  data: { a: null, rows: [], nodes: [], dlg: { show: false } },
  onLoad(q) { this.id = q.id; this.fresh = q.fresh === '1'; this.refresh(); },
  onUnload() { clearTimeout(this.urgeT); },
  refresh() {
    const a = S.getApproval(this.id);
    if (!a) return;
    const rows = F.fields[a.type].filter(f => f.type !== 'approver').map(f => {
      let v = a.values[f.key];
      if (f.type === 'switch') v = v ? '是' : '否';
      else if (f.unit && v !== '') v = v + ' ' + f.unit;
      if (f.key === 'amount') v = '¥' + Number(a.values.amount).toFixed(2);
      return { k: f.label, v: v === '' || v === undefined ? '—' : String(v) };
    });
    const nodes = a.nodes.map(n => Object.assign({}, n, { text: n.role === '发起申请' ? '已提交' : NODE_TEXT[n.status], ini: O.emp(n.who).ini, color: O.emp(n.who).color }));
    wx.setNavigationBarTitle({ title: a.typeName + '审批' });
    this.setData({ a, rows, nodes });
  },
  async decide(agree) {
    const comment = await ask.text(this, {
      title: agree ? '同意并填写意见' : '驳回原因',
      placeholder: agree ? '审批意见（选填），如：同意，注意交接' : '请写明驳回原因，申请人会收到通知',
      required: !agree
    });
    if (comment === null) return;
    const r = S.act(this.id, agree, comment, O.ME);
    if (!r) return wx.showToast({ title: '这张单已经被处理了', icon: 'none' });
    wx.showToast({ title: agree ? '已同意' : '已驳回', icon: 'success' });
    this.refresh();
    const ch = this.getOpenerEventChannel && this.getOpenerEventChannel();
    ch && ch.emit && ch.emit('changed', { id: this.id, status: r.status });
  },
  agree() { this.decide(true); },
  reject() { this.decide(false); },
  onDlgOk(e) { ask.ok(this, e); },
  onDlgCancel() { ask.cancel(this); },
  withdraw() {
    wx.showModal({
      title: '撤回申请', content: '撤回后审批人将不再处理，可以修改后重新发起。', confirmText: '撤回', confirmColor: '#f53f3f',
      success: r => {
        if (!r.confirm) return;
        S.withdraw(this.id);
        this.refresh();
        wx.showToast({ title: '已撤回', icon: 'none' });
      }
    });
  },
  // 催办：模拟当前审批人收到提醒后处理（同事那一端在真实系统里是另一台手机）
  urge() {
    const cur = this.data.a.nodes.find(n => n.status === 'pending');
    if (!cur) return;
    wx.showToast({ title: '已提醒 ' + cur.whoName, icon: 'none' });
    this.setData({ urging: true });
    this.urgeT = setTimeout(() => {
      S.act(this.id, true, cur.role === '人力备案' ? '已备案' : (cur.role === '财务复核' ? '票据核对无误' : '同意'), cur.who);
      this.setData({ urging: false });
      this.refresh();
    }, 1500);
  },
  again() { wx.redirectTo({ url: '/pages/apply/apply?type=' + this.data.a.type }); },
  copyNo() { wx.setClipboardData({ data: this.data.a.id }); }
});
