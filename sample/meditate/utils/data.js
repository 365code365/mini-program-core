// 课程目录：每节配一段内置音频（25 秒示范片段，真实课程由服务端下发完整音频）
const CATS = [
  { key: 'all', name: '全部' },
  { key: 'fav', name: '收藏' },
  { key: 'intro', name: '入门' },
  { key: 'sleep', name: '助眠' },
  { key: 'relax', name: '减压' },
  { key: 'focus', name: '专注' },
  { key: 'emotion', name: '情绪' },
  { key: 'body', name: '身体' }
];

const AUDIO = { bowl: '/audio/bowl.mp3', pad: '/audio/pad.mp3', piano: '/audio/piano.mp3' };

function ch(title, audio, mins) {
  return { title, audio: AUDIO[audio], mins };
}

const COURSES = [
  {
    id: 'c1', cat: 'intro', title: '七天正念入门', cover: '/images/cover/intro.jpg', teacher: '林予安',
    teacherIntro: '正念减压（MBSR）认证导师，带领团体练习 8 年', level: '零基础', learners: 128640,
    desc: '每天十分钟，从觉察呼吸开始，学会把注意力带回当下。适合第一次接触冥想的你。',
    chapters: [ch('第一天：找到呼吸', 'bowl', 8), ch('第二天：身体的重量', 'pad', 9), ch('第三天：和念头相处', 'piano', 10),
      ch('第四天：声音冥想', 'bowl', 10), ch('第五天：行走中的觉察', 'pad', 12), ch('第六天：善待自己', 'piano', 12), ch('第七天：把正念带进生活', 'bowl', 15)]
  },
  {
    id: 'c2', cat: 'sleep', title: '深度睡眠引导', cover: '/images/cover/sleep.jpg', teacher: '周沐言',
    teacherIntro: '国家二级心理咨询师，专注失眠认知行为干预', level: '适合入睡困难', learners: 236512,
    desc: '跟随舒缓的引导语放松全身，松开白天的紧绷，自然地滑入睡眠。建议睡前躺在床上收听。',
    chapters: [ch('睡前身体放松', 'pad', 15), ch('数息入眠', 'piano', 18), ch('月光下的海边', 'pad', 20), ch('半夜醒来时', 'bowl', 12)]
  },
  {
    id: 'c3', cat: 'relax', title: '十分钟减压', cover: '/images/cover/relax.jpg', teacher: '林予安',
    teacherIntro: '正念减压（MBSR）认证导师，带领团体练习 8 年', level: '午休 / 通勤', learners: 98213,
    desc: '工作间隙的快速充电：放下手机，用十分钟让心跳和呼吸慢下来。',
    chapters: [ch('三分钟呼吸空间', 'bowl', 3), ch('肩颈放松', 'pad', 8), ch('给压力命名', 'piano', 10)]
  },
  {
    id: 'c4', cat: 'focus', title: '专注力训练营', cover: '/images/cover/focus.jpg', teacher: '陈一帆',
    teacherIntro: '前互联网产品经理，研究注意力管理与深度工作', level: '进阶', learners: 54071,
    desc: '用数息、单点专注和开放觉察三种方法，训练你在纷扰中稳住注意力。',
    chapters: [ch('数息练习', 'piano', 10), ch('烛光凝视', 'bowl', 10), ch('开放觉察', 'pad', 12), ch('专注工作前的准备', 'piano', 6)]
  },
  {
    id: 'c5', cat: 'emotion', title: '情绪急救包', cover: '/images/cover/emotion.jpg', teacher: '周沐言',
    teacherIntro: '国家二级心理咨询师，专注失眠认知行为干预', level: '情绪波动时', learners: 77420,
    desc: '焦虑、烦躁、低落袭来时，用 RAIN 四步法接住情绪，而不是被它推着走。',
    chapters: [ch('焦虑时的着陆练习', 'bowl', 6), ch('RAIN 四步法', 'pad', 12), ch('慈心冥想', 'piano', 15)]
  },
  {
    id: 'c6', cat: 'body', title: '身体扫描', cover: '/images/cover/body.jpg', teacher: '陈一帆',
    teacherIntro: '前互联网产品经理，研究注意力管理与深度工作', level: '全阶段', learners: 61308,
    desc: '从脚尖到头顶逐一觉察身体的感受，释放积压在肌肉里的紧张。',
    chapters: [ch('从脚开始', 'pad', 10), ch('腹部与呼吸', 'bowl', 12), ch('完整身体扫描', 'piano', 25)]
  }
];

const NOISES = [
  { key: 'rain', name: '雨声', src: '/audio/rain.wav', color: '#5b8def' },
  { key: 'waves', name: '海浪', src: '/audio/waves.wav', color: '#3fb6c9' },
  { key: 'fire', name: '篝火', src: '/audio/fire.wav', color: '#f08a4b' }
];

// 睡前电台：走背景音频，切页、锁屏继续播
const RADIO = [
  { title: '雨夜钢琴', singer: '静心电台', src: AUDIO.piano, cover: '/images/cover/sleep.jpg' },
  { title: '温暖和弦', singer: '静心电台', src: AUDIO.pad, cover: '/images/cover/relax.jpg' },
  { title: '颂钵与低吟', singer: '静心电台', src: AUDIO.bowl, cover: '/images/cover/focus.jpg' }
];

const MOODS = [
  { key: 'calm', name: '平静', color: '#6cc3a8' },
  { key: 'happy', name: '开心', color: '#f5b54a' },
  { key: 'tired', name: '疲惫', color: '#9aa3c0' },
  { key: 'anxious', name: '焦虑', color: '#e88a6a' },
  { key: 'low', name: '低落', color: '#7c86d9' }
];

const MOOD_TAGS = ['工作', '学习', '家人', '朋友', '运动', '睡眠', '天气', '独处'];

function course(id) {
  return COURSES.find(c => c.id === id) || null;
}

module.exports = { CATS, COURSES, NOISES, RADIO, MOODS, MOOD_TAGS, course };
