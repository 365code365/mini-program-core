// 跳转封装：wx.navigateToMiniProgram + 记一条跳转记录（成功 / 失败原因），两个页面共用
const LOG_KEY = 'hub_jump_log';

// 精选：本仓库里能互相跳的几个示例。path 写各自真实存在的页面
const FEATURED = [
  { appId: 'wxaaf1983a0caced0c', name: '光影放映厅', description: '热映 / 待映 / 高分榜，详情页能跳云听音乐听原声', path: 'pages/index/index', color: '#ff4e6a' },
  { appId: 'wxc0bec341b9d3e883', name: '云听音乐', description: '带歌单 id 打开：直接进《星际回响》原声歌单', path: 'pages/index/index?playlistId=102', color: '#f04c4c' },
  { appId: 'wx6158cbd043d9faf1', name: '拾光图库', description: '双列瀑布流图库，打开指定作品详情', path: 'pages/detail/detail?id=8', color: '#3b82f6' },
  { appId: 'wxb16e4e02feb8a6da', name: '光影电影票', description: '在线选座购票（cinema 示例）', path: 'pages/index/index', color: '#f0384d' },
  { appId: 'wx78760257acd8e0fc', name: '悦听音乐', description: '本地曲库播放器（music 示例）', path: 'pages/discover/discover', color: '#ff3a5c' },
  { appId: 'wx3b7d9f1a2c4e6b80', name: '头条资讯', description: '长列表 / 视频 / 富文本（news-app 示例）', path: '', color: '#d43c33' }
];

function logs() { return wx.getStorageSync(LOG_KEY) || []; }

function pushLog(entry) {
  const list = [entry].concat(logs()).slice(0, 20);
  wx.setStorageSync(LOG_KEY, list);
  return list;
}

function two(n) { return (n < 10 ? '0' : '') + n; }
function now() {
  const d = new Date();
  return two(d.getHours()) + ':' + two(d.getMinutes()) + ':' + two(d.getSeconds());
}

/**
 * 打开另一个小程序。extraData 总会带上 from / ts，对方在 App.onLaunch / onShow 的 options.referrerInfo.extraData 里读到。
 * 返回 Promise：resolve 跳转成功，reject({errMsg})；两种结果都记进跳转记录，onLog(list) 回调最新记录
 */
function open(target, extra, onLog) {
  const extraData = Object.assign({ from: 'mini-hub', fromName: '小程序跳转中心', ts: Date.now() }, extra || {});
  const base = { name: target.name || target.appId, appId: target.appId, path: target.path || '', time: now() };
  return new Promise((resolve, reject) => {
    wx.navigateToMiniProgram({
      appId: target.appId,
      path: target.path || '',
      extraData,
      envVersion: target.envVersion || 'release',
      success(res) {
        const list = pushLog(Object.assign(base, { ok: true, msg: (res && res.errMsg) || 'navigateToMiniProgram:ok' }));
        if (onLog) onLog(list);
        resolve(res);
      },
      fail(err) {
        const msg = (err && err.errMsg) || 'navigateToMiniProgram:fail';
        const list = pushLog(Object.assign(base, { ok: false, msg }));
        if (onLog) onLog(list);
        wx.showModal({ title: '没能打开「' + base.name + '」', content: msg, showCancel: false });
        reject(err);
      }
    });
  });
}

function clearLogs() { wx.removeStorageSync(LOG_KEY); }

module.exports = { FEATURED, open, logs, clearLogs };
