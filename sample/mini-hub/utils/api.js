// 演示数据接口（tools/mini-store/demo_api.py）。测试时可以 wx.setStorageSync('demoApiBase', 'http://127.0.0.1:端口') 换
const BASE = wx.getStorageSync('demoApiBase') || 'http://64.83.22.217:9630';

// GET 参数自己拼：空值不带（服务器把 q= 当成「搜空串」，与不带 q 语义不同）
function query(params) {
  const ps = [];
  Object.keys(params || {}).forEach((k) => {
    const v = params[k];
    if (v === undefined || v === null || v === '') return;
    ps.push(encodeURIComponent(k) + '=' + encodeURIComponent(v));
  });
  return ps.length ? '?' + ps.join('&') : '';
}

/**
 * 发 GET，返回 Promise（resolve 解析好的 JSON）。
 * opts.loading：true 或文字 → 期间显示 wx.showLoading（首屏用；翻页、下拉刷新别用，会挡住手势）
 * opts.silent：失败不弹 toast（页面自己画「加载失败 · 重试」时用，免得一次失败提示两遍）
 */
function request(path, params, opts) {
  opts = opts || {};
  const url = (/^https?:\/\//.test(path) ? path : BASE + path) + query(params);
  if (opts.loading) wx.showLoading({ title: typeof opts.loading === 'string' ? opts.loading : '加载中', mask: true });
  return new Promise((resolve, reject) => {
    wx.request({
      url,
      method: 'GET',
      timeout: opts.timeout || 10000,
      success(res) {
        let data = res.data;
        // 有的宿主在 Content-Type 不认识时给字符串：自己再解析一次
        if (typeof data === 'string') { try { data = JSON.parse(data); } catch (e) { /* 留原样 */ } }
        if (res.statusCode >= 200 && res.statusCode < 300) resolve(data);
        else reject({ errMsg: 'HTTP ' + res.statusCode + (data && data.error ? '：' + data.error : ''), statusCode: res.statusCode });
      },
      fail(err) { reject({ errMsg: (err && err.errMsg) || 'request:fail' }); },
      complete() { if (opts.loading) wx.hideLoading(); }
    });
  }).catch((err) => {
    if (!opts.silent) wx.showToast({ title: err.statusCode === 404 ? '内容不存在' : '网络不给力，请稍后重试', icon: 'none' });
    throw err;
  });
}

// 站内相对地址（如商店的 iconUrl「/api/apps/x/icon?v=3」）补成绝对地址
function abs(url) {
  if (!url) return '';
  return /^https?:\/\//.test(url) ? url : BASE + url;
}

module.exports = { BASE, request, abs, query };
