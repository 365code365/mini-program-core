// 二级联动的数据
const SUB = {
  水果: ['苹果', '香蕉', '橙子', '葡萄'],
  蔬菜: ['白菜', '萝卜'],
  饮品: ['咖啡', '奶茶', '果汁']
};
const TOP = Object.keys(SUB);

Page({
  data: {
    cities: [{ id: 1, name: '北京' }, { id: 2, name: '上海' }, { id: 3, name: '广州' }, { id: 4, name: '深圳' }, { id: 5, name: '杭州' }],
    cityIndex: 1,
    multiRange: [TOP, SUB[TOP[0]]],
    multiIndex: [0, 0],
    time: '12:30',
    date: '2026-10-21',
    month: '2026-10',
    region: ['广东省', '深圳市', '南山区'],
    cup: 1,
    log: []
  },
  push(line) {
    const log = [line].concat(this.data.log).slice(0, 8);
    this.setData({ log });
  },
  type(v) {
    return Array.isArray(v) ? 'array' : typeof v;
  },
  onCity(e) {
    this.setData({ cityIndex: e.detail.value });
    this.push('selector change ' + e.detail.value + '（' + this.type(e.detail.value) + '）');
  },
  onColumn(e) {
    const { column, value } = e.detail;
    this.push('columnchange column=' + column + ' value=' + value);
    if (column === 0) {
      // 第一列变了：换第二列的选项并把选中项夹回 0
      this.setData({ 'multiRange[1]': SUB[TOP[value]], multiIndex: [value, 0] });
    }
  },
  onMulti(e) {
    this.setData({ multiIndex: e.detail.value });
    this.push('multiSelector change ' + JSON.stringify(e.detail.value) + '（' + this.type(e.detail.value) + '）');
  },
  onTime(e) {
    this.setData({ time: e.detail.value });
    this.push('time change "' + e.detail.value + '"');
  },
  onDate(e) {
    this.setData({ date: e.detail.value });
    this.push('date change "' + e.detail.value + '"');
  },
  onMonth(e) {
    this.setData({ month: e.detail.value });
    this.push('date(fields=month) change "' + e.detail.value + '"');
  },
  onRegion(e) {
    this.setData({ region: e.detail.value });
    this.push('region change ' + JSON.stringify(e.detail.value));
  },
  onCup(e) {
    this.setData({ cup: e.detail.value });
    this.push('默认触发视图 change ' + e.detail.value);
  },
  onCancel() {
    this.push('cancel');
  }
});
