// 深色模式：自管理主题（引擎不读 app.json 的 darkmode / theme.json，见 README 说明）。
// 设置里三选一：浅色 / 深色 / 跟随系统；跟随系统时读 getSystemInfoSync().theme，并由 app.js 的 wx.onThemeChange 刷新。
// 每页根节点写 class="page {{theme}}"，样式在 app.wxss 的 .dark 后代选择器里。
const store = require('./store.js');

const NAV = {
  light: { frontColor: '#000000', backgroundColor: '#f6f5fb' },
  dark: { frontColor: '#ffffff', backgroundColor: '#17182c' }
};

function current() {
  const mode = store.settings().theme;
  if (mode === 'system') {
    const app = getApp();
    return (app && app.globalData.sysTheme) === 'dark' ? 'dark' : 'light';
  }
  return mode === 'dark' ? 'dark' : 'light';
}

// tabIndex < 0：不是 tab 页
function apply(page, tabIndex) {
  const t = current();
  if (page.data.theme !== t) page.setData({ theme: t });
  wx.setNavigationBarColor(Object.assign({ animation: { duration: 0 } }, NAV[t]));
  wx.setBackgroundColor({ backgroundColor: NAV[t].backgroundColor });
  if (tabIndex >= 0 && typeof page.getTabBar === 'function' && page.getTabBar()) {
    page.getTabBar().setData({ selected: tabIndex, dark: t === 'dark' });
  }
  return t;
}

module.exports = { current, apply };
