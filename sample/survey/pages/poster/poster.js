const data = require('../../utils/data.js');
// 分享海报：canvas 2d 现画，导出图片后存相册；存不了（桌面端没有相册 / 用户拒绝）就预览大图让用户长按保存
const W = 300;
const H = 440;
// 按宽度折行（中文逐字量），最多 maxLines 行，超出加省略号
function wrap(ctx, text, width, maxLines) {
  const lines = [];
  let line = '';
  for (const ch of text) {
    if (ctx.measureText(line + ch).width > width && line) {
      lines.push(line);
      line = ch;
      if (lines.length === maxLines) break;
    } else line += ch;
  }
  if (lines.length < maxLines && line) lines.push(line);
  else if (lines.length === maxLines && line) lines[maxLines - 1] = lines[maxLines - 1].slice(0, -1) + '…';
  return lines;
}
Page({
  data: { sv: null, ready: false, saving: false, tempPath: '', topText: '' },
  onLoad(q) {
    const sv = data.getSurvey(q.id || 'q1');
    this.sv = sv;
    // 第一题的领先选项（选择题才有）
    const q0 = sv.questions[0];
    let topText = '';
    if (q0.options && q0.base) {
      const total = Math.max(1, q0.base.reduce((a, b) => a + b, 0));
      const k = q0.base.indexOf(Math.max.apply(null, q0.base));
      topText = '目前领先：' + q0.options[k] + ' ' + Math.round(q0.base[k] * 100 / total) + '%';
    }
    this.setData({ sv, topText });
  },
  onReady() {
    wx.createSelectorQuery().select('#poster').fields({ node: true, size: true }).exec(res => {
      const c = res && res[0] && res[0].node;
      if (!c) { wx.showToast({ title: '海报生成失败', icon: 'none' }); return; }
      const dpr = wx.getSystemInfoSync().pixelRatio || 2;
      c.width = W * dpr;
      c.height = H * dpr;
      const ctx = c.getContext('2d');
      ctx.scale(dpr, dpr);
      this.canvas = c;
      this.draw(ctx);
      this.setData({ ready: true });
    });
  },
  draw(ctx) {
    const sv = this.sv;
    const g = ctx.createLinearGradient(0, 0, W, H);
    g.addColorStop(0, '#8E7CFF');
    g.addColorStop(1, '#5A48D6');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, W, H);
    // 装饰圆
    ctx.fillStyle = 'rgba(255,255,255,0.12)';
    ctx.beginPath(); ctx.arc(W - 30, 40, 70, 0, Math.PI * 2); ctx.fill();
    ctx.beginPath(); ctx.arc(20, H - 60, 50, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = '#ffffff';
    ctx.font = 'bold 22px sans-serif';
    ctx.fillText('问卷星球', 24, 44);
    ctx.font = '12px sans-serif';
    ctx.fillStyle = 'rgba(255,255,255,0.85)';
    ctx.fillText('邀你一起来' + (sv.kind === '投票' ? '投票' : '填问卷'), 24, 66);
    // 白卡片
    const x = 20, y = 90, w = W - 40, h = 250;
    ctx.fillStyle = '#ffffff';
    ctx.beginPath();
    ctx.moveTo(x + 14, y); ctx.lineTo(x + w - 14, y); ctx.arcTo(x + w, y, x + w, y + 14, 14);
    ctx.lineTo(x + w, y + h - 14); ctx.arcTo(x + w, y + h, x + w - 14, y + h, 14);
    ctx.lineTo(x + 14, y + h); ctx.arcTo(x, y + h, x, y + h - 14, 14);
    ctx.lineTo(x, y + 14); ctx.arcTo(x, y, x + 14, y, 14);
    ctx.closePath(); ctx.fill();
    ctx.fillStyle = sv.kind === '投票' ? '#FF7A1A' : '#6C5CE7';
    ctx.font = 'bold 11px sans-serif';
    ctx.fillText(sv.kind, x + 18, y + 28);
    // 标题 1~3 行，说明紧跟其后；人数固定贴卡片底部
    ctx.fillStyle = '#1f2329';
    ctx.font = 'bold 18px sans-serif';
    const tl = wrap(ctx, sv.title, w - 36, 3);
    tl.forEach((l, i) => ctx.fillText(l, x + 18, y + 58 + i * 26));
    let cy = y + 58 + (tl.length - 1) * 26 + 28;
    ctx.fillStyle = '#8a8f99';
    ctx.font = '12px sans-serif';
    const dl = wrap(ctx, sv.desc, w - 36, 2);
    dl.forEach((l, i) => ctx.fillText(l, x + 18, cy + i * 18));
    cy += (dl.length - 1) * 18 + 24;
    if (this.data.topText) {
      ctx.fillStyle = '#FF7A1A';
      ctx.fillText(this.data.topText, x + 18, cy);
    }
    ctx.fillStyle = '#6C5CE7';
    ctx.font = 'bold 26px sans-serif';
    ctx.fillText(data.fmtNum(sv.joined), x + 18, y + h - 40);
    ctx.fillStyle = '#8a8f99';
    ctx.font = '12px sans-serif';
    ctx.fillText('人已参与 · ' + sv.questions.length + ' 题 · 截止 ' + sv.deadline, x + 18, y + h - 16);
    ctx.fillStyle = '#ffffff';
    ctx.font = '13px sans-serif';
    ctx.fillText('微信搜索「问卷星球」', 24, H - 52);
    ctx.fillStyle = 'rgba(255,255,255,0.8)';
    ctx.font = '11px sans-serif';
    ctx.fillText('问卷编号 ' + sv.id.toUpperCase() + ' · 由 ' + sv.author + ' 发起', 24, H - 32);
  },
  export(cb) {
    if (this.data.tempPath) return cb(this.data.tempPath);
    wx.canvasToTempFilePath({
      canvas: this.canvas,
      fileType: 'jpg',
      quality: 0.9,
      success: r => { this.setData({ tempPath: r.tempFilePath }); cb(r.tempFilePath); },
      fail: () => { this.setData({ saving: false }); wx.showToast({ title: '导出图片失败', icon: 'none' }); }
    });
  },
  onSave() {
    if (!this.data.ready || this.data.saving) return;
    this.setData({ saving: true });
    this.export(path => {
      wx.saveImageToPhotosAlbum({
        filePath: path,
        success: () => { this.setData({ saving: false }); wx.showToast({ title: '已保存到相册', icon: 'success' }); },
        fail: () => {
          this.setData({ saving: false });
          // 没有相册权限 / 桌面端：打开大图，长按保存或转发
          wx.showModal({
            title: '无法直接保存',
            content: '没有相册权限。将打开海报大图，长按即可保存或发送给好友。',
            confirmText: '打开大图',
            confirmColor: '#6C5CE7',
            success: r => { if (r.confirm) wx.previewImage({ urls: [path], current: path }); }
          });
        }
      });
    });
  },
  onCopyLink() {
    wx.setClipboardData({ data: '【问卷星球】' + this.sv.title + ' 微信搜索「问卷星球」，输入编号 ' + this.sv.id.toUpperCase() + ' 参与' });
  },
  onShareAppMessage() {
    return { title: this.sv.title, path: '/pages/intro/intro?id=' + this.sv.id };
  }
});
