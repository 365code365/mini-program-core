// 存档：wx.getFileSystemManager() 写 `${wx.env.USER_DATA_PATH}/minesweeper/save.json`
// （最佳用时、胜负统计、没下完的那一局）。文件系统不可用 / 读写失败时退回 storage，玩法不受影响。
// 为什么不直接用 storage：这一款专门覆盖文件 API（mkdir / access / writeFile / readFile / stat），
// 而且没下完的棋盘一格一个字符，高级 480 字节，放文件里更像真实游戏的存档。
var fs = null, DIR = '', FILE = '', KEY = 'minesweeper.save';
try {
  fs = wx.getFileSystemManager ? wx.getFileSystemManager() : null;
  var root = (wx.env && wx.env.USER_DATA_PATH) || '';
  if (!fs || !root) fs = null;
  else { DIR = root + '/minesweeper'; FILE = DIR + '/save.json'; }
} catch (e) { fs = null; }

var info = { via: fs ? 'file' : 'storage', writes: 0, bytes: 0, error: '' };

function ensureDir() {
  try { fs.accessSync(DIR); } catch (e) { fs.mkdirSync(DIR, true); }
}
function read() {
  var text = '';
  if (fs) {
    try {
      fs.accessSync(FILE);
      text = fs.readFileSync(FILE, 'utf8');
    } catch (e) {
      // 第一次玩还没有文件：正常情况，不算错
      if (!/no such file|not exist|ENOENT/i.test(String(e && (e.errMsg || e.message || e)))) info.error = String(e && (e.errMsg || e.message || e));
    }
  }
  if (!text) { try { text = wx.getStorageSync(KEY) || ''; } catch (e) {} }
  try { return text ? JSON.parse(text) : {}; } catch (e) { return {}; }
}
function write(data) {
  var text = JSON.stringify(data);
  info.bytes = text.length;
  if (fs) {
    try {
      ensureDir();
      fs.writeFileSync(FILE, text, 'utf8');
      info.writes++; info.via = 'file';
      return;
    } catch (e) {
      info.error = String(e && (e.errMsg || e.message || e));
      info.via = 'storage';
    }
  }
  try { wx.setStorageSync(KEY, text); info.writes++; } catch (e) {}
}
// 文件大小（调试面板与 __play() 用）：stat 一次
function size() {
  if (!fs) return -1;
  try { var s = fs.statSync(FILE); return s && s.size !== undefined ? s.size : -1; } catch (e) { return -1; }
}

module.exports = { read: read, write: write, size: size, info: info, path: function () { return FILE; } };
