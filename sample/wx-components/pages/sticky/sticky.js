const app = getApp();
const GROUPS = ['水果', '蔬菜', '饮品', '零食', '粮油'].map((name, g) => ({
  name, items: Array.from({ length: 6 }, (_, i) => `${name} ${i + 1}`)
}));
Page({
  data: { groups: GROUPS, tabs: ['推荐', '附近', '关注'], tab: 0, log: [] },
  onTab(e) { this.setData({ tab: Number(e.currentTarget.dataset.i) }); },
  onStick(e) {
    // detail.isStickOnTop 是 boolean
    app.note(this, `stickontopchange ${e.currentTarget.dataset.name} isStickOnTop=${e.detail.isStickOnTop} (${typeof e.detail.isStickOnTop})`);
  },
  onTapHead(e) { app.note(this, `点到吸住的头：${e.currentTarget.dataset.name}`); }
});
