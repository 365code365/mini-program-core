App({
  onLaunch() {
    if (!wx.getStorageSync('jobs_resume')) {
      wx.setStorageSync('jobs_resume', {
        name: '林晓', gender: 1, age: '27', phone: '138****6621', city: '杭州',
        edu: 1, school: '浙江工业大学', major: '计算机科学与技术',
        expYears: 2, position: '前端开发工程师', salary: 3,
        intro: '4 年 Web 与小程序开发经验，主导过日活百万级电商小程序的性能优化，首屏耗时下降 38%。熟悉 React、TypeScript 与工程化建设。'
      });
    }
  },
  globalData: {}
});
