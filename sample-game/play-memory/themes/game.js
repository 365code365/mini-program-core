// 分包 themes 的入口：wx.loadSubpackage({ name: 'themes' }) 成功时执行一次，把两套卡面登记进主包的主题表。
var reg = GameGlobal.__memThemes = GameGlobal.__memThemes || {};
reg.jieqi = require('./jieqi.js');
reg.zodiac = require('./zodiac.js');
console.log('卡面分包已加载：' + reg.jieqi.name + ' ' + reg.jieqi.items.length + ' 张、' + reg.zodiac.name + ' ' + reg.zodiac.items.length + ' 张');
