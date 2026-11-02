// 通用工具：日期、电话 / 上传 / 选图这些需要宿主的能力的降级封装。
const pad = (n) => (n < 10 ? '0' + n : '' + n);
function ymd(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function stamp(d) { d = d || new Date(); return ymd(d) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes()); }
function addDays(n) { const d = new Date(); d.setDate(d.getDate() + n); return ymd(d); }

// 上门时段：08:00 ~ 18:00 每半小时
function timeSlots() {
  const out = [];
  for (let h = 8; h <= 18; h++) { out.push(pad(h) + ':00'); if (h < 18) out.push(pad(h) + ':30'); }
  return out;
}

function callPhone(phone, title) {
  wx.makePhoneCall({
    phoneNumber: phone.replace(/-/g, ''),
    fail: () => wx.showModal({
      title: title || '联系电话', content: phone, confirmText: '复制号码',
      success: (r) => { if (r.confirm) wx.setClipboardData({ data: phone }); }
    })
  });
}

// 选现场照片：桌面端 / 没有相册时 chooseMedia 走 fail，给出示例照片入口
const SAMPLES = ['/images/photo/kitchen.jpg', '/images/photo/bathroom.jpg'];
function choosePhotos(left, cb) {
  wx.chooseMedia({
    count: left, mediaType: ['image'], sourceType: ['album', 'camera'], sizeType: ['compressed'],
    success: (r) => cb(r.tempFiles.map((f) => f.tempFilePath)),
    fail: (err) => {
      // 桌面端也回 cancel，分不清「用户取消」和「没有相册」：只提示，页面上常驻「示例图」入口
      if (/cancel/.test(err.errMsg || '')) { wx.showToast({ title: '没有选择照片，可点「示例图」', icon: 'none' }); return; }
      wx.showModal({
        title: '无法打开相册', content: '可以先用示例照片说明现场情况（厨房油污 / 卫生间水垢），师傅上门后再拍照确认。',
        confirmText: '用示例图', cancelText: '不用了',
        success: (m) => { if (m.confirm) cb(SAMPLES.slice(0, left)); }
      });
    }
  });
}

// 上传照片：uploadFile 失败（无网络 / 无上传服务）就把本地路径留在订单里，标记为「本地保存」
function uploadPhotos(paths, done) {
  if (!paths.length) { done([]); return; }
  const out = [];
  let left = paths.length;
  paths.forEach((p, i) => {
    wx.uploadFile({
      url: 'https://api.haoguanjia.cn/v1/upload', filePath: p, name: 'file', formData: { scene: 'order' },
      success: (r) => { let url = p; try { url = JSON.parse(r.data).url || p; } catch (e) { /* 回包不是 JSON：用本地路径 */ } out[i] = { path: url, local: url === p }; },
      fail: () => { out[i] = { path: p, local: true }; },
      complete: () => { if (--left === 0) done(out); }
    });
  });
}

function maskPhone(p) { return p ? p.slice(0, 3) + '****' + p.slice(7) : ''; }

module.exports = { pad, ymd, stamp, addDays, timeSlots, callPhone, choosePhotos, uploadPhotos, maskPhone };
