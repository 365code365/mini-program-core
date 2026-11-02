// 申报草稿：存成用户目录下的 draft.json（wx.getFileSystemManager），而不是 storage ——
// 草稿里带着材料文件路径与整份表单，体量随材料数涨，文件更贴近「草稿箱」的语义，也能单独删。
const fs = wx.getFileSystemManager();
const PATH = `${wx.env.USER_DATA_PATH}/draft.json`;

function save(draft, cb) {
  const data = JSON.stringify(Object.assign({}, draft, { savedAt: Date.now() }));
  fs.writeFile({
    filePath: PATH, data, encoding: 'utf8',
    success: () => cb && cb(null),
    fail: (e) => cb && cb(e.errMsg || '写入失败')
  });
}

// 先 access 判断有没有，再 readFile；文件坏了（JSON 解析失败）当作没有草稿并删掉
function load(cb) {
  fs.access({
    path: PATH,
    success: () => {
      fs.readFile({
        filePath: PATH, encoding: 'utf8',
        success: (r) => {
          let d = null;
          try { d = JSON.parse(r.data); } catch (e) { remove(); }
          cb(d && d.form ? d : null);
        },
        fail: () => cb(null)
      });
    },
    fail: () => cb(null)
  });
}

function remove(cb) {
  fs.unlink({ filePath: PATH, success: () => cb && cb(true), fail: () => cb && cb(false) });
}

module.exports = { save, load, remove, PATH };
