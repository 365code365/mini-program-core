App({
  globalData: { log: [] },
  // 各页把事件记一笔，页面底部显示最近几条（方便在真机上核对 e.detail 的字段与类型）
  note(page, text) {
    const line = `${new Date().toTimeString().slice(0, 8)} ${text}`;
    this.globalData.log.unshift(line);
    if (this.globalData.log.length > 30) this.globalData.log.length = 30;
    if (page && page.setData) page.setData({ log: this.globalData.log.slice(0, 6) });
  }
});
