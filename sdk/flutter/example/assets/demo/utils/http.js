// 请求封装：Promise 化 + 统一错误 + 请求日志。
// url 以 mock:// 开头走 utils/mock-server.js（离线可用），其余走真正的 wx.request。
const mock = require('./mock-server.js');

const logs = [];
const listeners = [];

function record(entry) {
  logs.unshift(entry);
  if (logs.length > 8) logs.pop();
  listeners.forEach((fn) => fn(logs.slice()));
}

/**
 * request({ url, method, data }) → Promise<data>
 * 非 2xx 与网络错误都 reject 成 Error（message 可以直接给用户看）
 */
function request(opts) {
  const started = Date.now();
  const method = (opts.method || 'GET').toUpperCase();
  const send = opts.url.indexOf('mock://') === 0 ? mock.request : wx.request;
  return new Promise((resolve, reject) => {
    send({
      url: opts.url,
      method,
      data: opts.data,
      header: opts.header,
      success(res) {
        const ms = Date.now() - started;
        record({ method, url: opts.url, status: res.statusCode, ms, ok: res.statusCode >= 200 && res.statusCode < 300 });
        if (res.statusCode >= 200 && res.statusCode < 300) {
          resolve(res.data);
        } else {
          reject(new Error('HTTP ' + res.statusCode + ((res.data && res.data.message) ? '：' + res.data.message : '')));
        }
      },
      fail(err) {
        record({ method, url: opts.url, status: '失败', ms: Date.now() - started, ok: false });
        reject(new Error(err.errMsg || 'request:fail'));
      }
    });
  });
}

/** 订阅请求日志（页面 onLoad 订阅、onUnload 退订） */
function onLog(fn) {
  listeners.push(fn);
  fn(logs.slice());
  return () => {
    const i = listeners.indexOf(fn);
    if (i >= 0) listeners.splice(i, 1);
  };
}

module.exports = { request, onLog, mock };
