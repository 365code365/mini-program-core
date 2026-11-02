// 选择审批人：发起审批页 navigateTo 时带 events.picked，这里选中后经 EventChannel 回传再返回。
Page({
  onSelect(e) {
    const emp = e.detail;
    const ch = this.getOpenerEventChannel && this.getOpenerEventChannel();
    if (ch && ch.emit) ch.emit('picked', { id: emp.id, name: emp.name, title: emp.title, deptName: emp.deptName });
    wx.navigateBack();
  }
});
