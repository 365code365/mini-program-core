//! 基准场景与夹具（设计 §12.2）。
//!
//! 夹具小程序以字符串常量写在这里，跑的时候生成到 `<out>/apps/`：**不进 `sample/`、不进仓库**。
//! `sample/` 是双端一致性的固化输入，往里加东西会让历史像素基线跟着变；
//! 基准只需要「可复现的负载」，放在 target 下由 `clean-target.sh` 清掉就够了。
//!
//! R1 用仓库里现成的示例小程序（只当黑盒运行，不读也不改源码）。
use std::path::{Path, PathBuf};

/// 场景怎么驱动
#[derive(Clone, Debug)]
pub enum Drive {
    /// 每帧上移 `dy` 逻辑像素，共 `frames` 帧（手指拖动）
    Drag { dy: f32, frames: u32 },
    /// 空转 `frames` 帧（动画 / 定时器 / 覆盖层）
    Frames(u32),
}

/// 守卫：不过就判该场景失败（设计 §12.4）。防的是「全绿但没测到」
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Guard {
    /// 实际滚动距离 ≥ 请求距离的 90%（或已经滚到底）
    Scrolled,
    /// `animating` 的帧 ≥ 90%
    Animating,
    /// 重画过页面画布的帧 ≥ 30%
    Rerastered,
    /// 每帧 Modal 都弹着
    ModalVisible,
}

#[derive(Clone, Debug)]
pub struct Scene {
    /// S1 / R1 …
    pub id: &'static str,
    pub name: &'static str,
    /// 夹具名（生成在 `<out>/apps/<fixture>`）或仓库内的示例路径
    pub app: AppRef,
    pub route: &'static str,
    pub settle: f32,
    pub evals: &'static [&'static str],
    pub drive: Drive,
    pub scales: &'static [f32],
    pub guard: Guard,
    /// 衡量什么（写进 report.md 的场景说明）
    pub measures: &'static str,
}

#[derive(Clone, Debug)]
pub enum AppRef {
    Fixture(&'static str),
    Repo(&'static str),
}

impl Scene {
    pub fn app_path(&self, apps_dir: &Path) -> PathBuf {
        match self.app {
            AppRef::Fixture(name) => apps_dir.join(name),
            AppRef::Repo(p) => PathBuf::from(p),
        }
    }
}

/// R1 的 eval：sample-app 首页开屏 200ms 后弹「新人券」浮层（`showCoupon`），
/// 浮层是 `position: fixed` 的全屏遮罩，按设计会吃掉拖动、页面不滚 ——
/// settle 2 秒后直接拖的话 120 帧下来页面一像素都没动，测的是「对着遮罩空拖」。
/// 可行性报告 §2.1 里 R1 的 p50 0.5~0.8ms 就是这么测出来的。这里先把浮层关掉再拖。
const R1_CLOSE_COUPON: &[&str] = &["getCurrentPages()[0].setData({showCoupon:false})"];
const S5_MODAL: &[&str] = &["wx.showModal({title:'基准',content:'覆盖层直通帧'})"];

pub fn all() -> Vec<Scene> {
    vec![
        Scene {
            id: "S1", name: "scroll_list", app: AppRef::Fixture("scroll_list"), route: "pages/p/p",
            settle: 0.5, evals: &[], drive: Drive::Drag { dy: 8.0, frames: 300 }, scales: &[2.0, 3.0],
            guard: Guard::Scrolled, measures: "页面级滚动：合成帧成本 + 跨条带重画",
        },
        Scene {
            id: "S2", name: "anim_large", app: AppRef::Fixture("anim_large"), route: "pages/p/p",
            settle: 0.5, evals: &[], drive: Drive::Frames(300), scales: &[2.0],
            guard: Guard::Animating, measures: "大面积 transform / opacity 动画",
        },
        Scene {
            id: "S3", name: "heavy_paint", app: AppRef::Fixture("heavy_paint"), route: "pages/p/p",
            settle: 1.0, evals: &[], drive: Drive::Drag { dy: 8.0, frames: 300 }, scales: &[2.0, 3.0],
            guard: Guard::Scrolled, measures: "阴影 / 圆角 / 渐变 / 大图",
        },
        Scene {
            id: "S4", name: "full_redraw", app: AppRef::Fixture("full_redraw"), route: "pages/p/p",
            settle: 0.5, evals: &[], drive: Drive::Frames(300), scales: &[2.0],
            guard: Guard::Rerastered, measures: "整屏重绘（每 16ms setData 切根节点 class）",
        },
        Scene {
            id: "S5", name: "modal_overlay", app: AppRef::Fixture("scroll_list"), route: "pages/p/p",
            settle: 0.5, evals: S5_MODAL, drive: Drive::Frames(300), scales: &[2.0],
            guard: Guard::ModalVisible, measures: "覆盖层（Modal）弹着的帧",
        },
        Scene {
            id: "R1", name: "sample_index", app: AppRef::Repo("sample-app"), route: "pages/index/index",
            settle: 2.0, evals: R1_CLOSE_COUPON, drive: Drive::Drag { dy: 8.0, frames: 120 }, scales: &[2.0, 3.0],
            guard: Guard::Scrolled, measures: "真实页面滚动（sample-app 首页，关掉新人券浮层）",
        },
        Scene {
            // 设计 v1 的 R2 是 tea-app 分类页，但那一页在 375×667 下内容正好一屏
            // （MINI_SCROLL_LOG：scroll-view max_scroll=0，交给页面后页面也不可滚），
            // 拖 120 帧一像素都不动，测不到「嵌套 scroll-view 拖动」。换成确定性的夹具：
            // 根 view 里套一个定高 scroll-view（uni-app 产物的典型结构），拖动点落在它里面。
            id: "S6", name: "nested_scroll", app: AppRef::Fixture("nested_scroll"), route: "pages/p/p",
            settle: 0.5, evals: &[], drive: Drive::Drag { dy: 8.0, frames: 120 }, scales: &[2.0, 3.0],
            guard: Guard::Scrolled, measures: "嵌套 scroll-view 拖动（根 view 里套 scroll-view）",
        },
    ]
}

// ───────────────────────────── 夹具源码 ─────────────────────────────

const APP_JS: &str = "App({});\n";
const PAGE_JSON: &str = "{ \"usingComponents\": {} }\n";

fn app_json(title: &str) -> String {
    format!("{{ \"pages\": [\"pages/p/p\"], \"window\": {{ \"navigationBarTitleText\": \"{title}\" }} }}\n")
}

/// S1 / S5：300 张卡片（圆角、两行文字、纯色圆头像）
const LIST_JS: &str = r#"Page({
  data: { items: [] },
  onLoad: function () {
    var a = [];
    // 头像色用十六进制：引擎的颜色解析还不认 hsl()（与 Skyline 的差距，记在对齐清单里）
    var pal = ['#4a90e2', '#50c878', '#ff7a45', '#9254de', '#f759ab', '#13c2c2', '#faad14', '#2f54eb'];
    for (var i = 0; i < 300; i++) a.push({ id: i, title: '卡片标题 ' + i, sub: '第二行说明文字，用来撑出两行排版 #' + i, color: pal[i % 8] });
    this.setData({ items: a });
  }
});
"#;
const LIST_WXML: &str = r#"<view class="list">
  <view wx:for="{{items}}" wx:key="id" class="card">
    <view class="avatar" style="background-color: {{item.color}};"></view>
    <view class="body">
      <text class="title">{{item.title}}</text>
      <text class="sub">{{item.sub}}</text>
    </view>
  </view>
</view>
"#;
const LIST_WXSS: &str = r#"page { background-color: #f2f3f5; }
.list { padding: 16rpx 0; }
.card { display: flex; flex-direction: row; align-items: center; margin: 16rpx 24rpx; padding: 24rpx; background-color: #ffffff; border-radius: 24rpx; }
.avatar { width: 88rpx; height: 88rpx; border-radius: 44rpx; margin-right: 24rpx; }
.body { display: flex; flex-direction: column; flex: 1; }
.title { font-size: 30rpx; color: #222222; }
.sub { font-size: 24rpx; color: #888888; margin-top: 8rpx; }
"#;

/// S2：占视口约 70% 的块，渐变底 + 文字，无限循环的旋转 / 缩放 / 透明度
const ANIM_JS: &str = "Page({ data: {} });\n";
const ANIM_WXML: &str = r#"<view class="stage">
  <view class="block"><text class="label">大面积动画</text></view>
</view>
"#;
const ANIM_WXSS: &str = r#"page { background-color: #101820; }
.stage { display: flex; align-items: center; justify-content: center; height: 100vh; }
.block { width: 600rpx; height: 900rpx; border-radius: 32rpx; display: flex; align-items: center; justify-content: center;
  background: linear-gradient(135deg, #ff7a45 0%, #722ed1 100%); animation: spin 2s linear infinite; }
.label { font-size: 48rpx; color: #ffffff; }
@keyframes spin {
  0% { transform: rotate(0deg) scale(0.9); opacity: 0.6; }
  50% { transform: rotate(180deg) scale(1); opacity: 1; }
  100% { transform: rotate(360deg) scale(0.9); opacity: 0.6; }
}
"#;

/// S3 / S4：顶部大图 + 10 张带阴影 / 圆角 / 渐变的卡片
const HEAVY_WXML: &str = r#"<view class="root {{alt ? 'alt' : ''}}">
  <image class="hero" src="/images/big.png" mode="aspectFill"></image>
  <view wx:for="{{cards}}" wx:key="*this" class="card">
    <text class="ct">重绘卡片 {{item}}</text>
    <text class="cs">box-shadow + border-radius + linear-gradient</text>
  </view>
</view>
"#;
const HEAVY_WXSS: &str = r#"page { background-color: #eef1f6; }
.root { padding-bottom: 40rpx; background-color: #eef1f6; }
.root.alt { background-color: #fff7e6; }
.hero { width: 750rpx; height: 500rpx; }
.card { margin: 32rpx 32rpx 0 32rpx; padding: 40rpx; height: 260rpx; border-radius: 32rpx;
  box-shadow: 0 8rpx 32rpx rgba(0, 0, 0, 0.25);
  background: linear-gradient(180deg, #ffffff 0%, #e6f4ff 100%); display: flex; flex-direction: column; }
.root.alt .card { background: linear-gradient(180deg, #ffffff 0%, #fff1b8 100%); }
.ct { font-size: 34rpx; color: #1f1f1f; }
.cs { font-size: 24rpx; color: #8c8c8c; margin-top: 12rpx; }
"#;
const HEAVY_JS: &str = "Page({ data: { alt: false, cards: [1,2,3,4,5,6,7,8,9,10] } });\n";
/// S4：每 16ms 切一次根节点的 class（两套背景色），每帧都是整屏重绘
const REDRAW_JS: &str = r#"Page({
  data: { alt: false, cards: [1,2,3,4,5,6,7,8,9,10] },
  onLoad: function () { var self = this; this.timer = setInterval(function () { self.setData({ alt: !self.data.alt }); }, 16); },
  onUnload: function () { clearInterval(this.timer); }
});
"#;

/// S6：页面根 view 里套一个定高的 scroll-view，里面 200 行
const NESTED_JS: &str = r#"Page({
  data: { rows: [] },
  onLoad: function () {
    var a = [];
    for (var i = 0; i < 200; i++) a.push({ id: i, name: '列表行 ' + i, price: (i * 7) % 100 + 0.5 });
    this.setData({ rows: a });
  }
});
"#;
const NESTED_WXML: &str = r#"<view class="page">
  <view class="header"><text class="h1">嵌套滚动</text></view>
  <scroll-view scroll-y class="list">
    <view wx:for="{{rows}}" wx:key="id" class="row">
      <view class="dot"></view>
      <text class="name">{{item.name}}</text>
      <text class="price">¥{{item.price}}</text>
    </view>
  </scroll-view>
</view>
"#;
const NESTED_WXSS: &str = r#"page { background-color: #f7f7f7; }
.page { display: flex; flex-direction: column; height: 100vh; }
.header { height: 120rpx; display: flex; align-items: center; padding-left: 32rpx; background-color: #ffffff; }
.h1 { font-size: 36rpx; color: #111111; }
.list { height: 1100rpx; background-color: #ffffff; }
.row { display: flex; flex-direction: row; align-items: center; height: 96rpx; padding: 0 32rpx; border-bottom: 1rpx solid #eeeeee; }
.dot { width: 24rpx; height: 24rpx; border-radius: 12rpx; background-color: #52c41a; margin-right: 20rpx; }
.name { flex: 1; font-size: 28rpx; color: #333333; }
.price { font-size: 28rpx; color: #fa541c; }
"#;

/// 确定性的 1500×1000 渐变 + 噪声图（固定种子的 LCG：每次生成逐字节相同）
fn big_png(path: &Path) -> Result<(), String> {
    let (w, h) = (1500u32, 1000u32);
    let mut seed: u32 = 0x1234_5678;
    let mut img = image::RgbImage::new(w, h);
    for (x, y, px) in img.enumerate_pixels_mut() {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let n = (seed >> 24) as i32 - 128; // -128..127
        let r = (x * 255 / w) as i32 + n / 8;
        let g = (y * 255 / h) as i32 + n / 8;
        let b = (((x + y) * 255) / (w + h)) as i32 + n / 4;
        *px = image::Rgb([r.clamp(0, 255) as u8, g.clamp(0, 255) as u8, (255 - b).clamp(0, 255) as u8]);
    }
    img.save(path).map_err(|e| format!("写 {} 失败：{e}", path.display()))
}

fn write_app(dir: &Path, title: &str, js: &str, wxml: &str, wxss: &str) -> Result<(), String> {
    let page = dir.join("pages/p");
    std::fs::create_dir_all(&page).map_err(|e| format!("建目录 {} 失败：{e}", page.display()))?;
    let files: [(PathBuf, String); 6] = [
        (dir.join("app.js"), APP_JS.into()),
        (dir.join("app.json"), app_json(title)),
        (page.join("p.js"), js.into()),
        (page.join("p.json"), PAGE_JSON.into()),
        (page.join("p.wxml"), wxml.into()),
        (page.join("p.wxss"), wxss.into()),
    ];
    for (p, text) in files {
        std::fs::write(&p, text).map_err(|e| format!("写 {} 失败：{e}", p.display()))?;
    }
    Ok(())
}

/// 把全部夹具写到 `apps_dir`（已存在就覆盖）
pub fn generate(apps_dir: &Path) -> Result<(), String> {
    write_app(&apps_dir.join("scroll_list"), "S1 scroll_list", LIST_JS, LIST_WXML, LIST_WXSS)?;
    write_app(&apps_dir.join("anim_large"), "S2 anim_large", ANIM_JS, ANIM_WXML, ANIM_WXSS)?;
    write_app(&apps_dir.join("nested_scroll"), "S6 nested_scroll", NESTED_JS, NESTED_WXML, NESTED_WXSS)?;
    for (name, js) in [("heavy_paint", HEAVY_JS), ("full_redraw", REDRAW_JS)] {
        let dir = apps_dir.join(name);
        write_app(&dir, name, js, HEAVY_WXML, HEAVY_WXSS)?;
        let images = dir.join("images");
        std::fs::create_dir_all(&images).map_err(|e| format!("建目录 {} 失败：{e}", images.display()))?;
        big_png(&images.join("big.png"))?;
    }
    Ok(())
}

impl Drive {
    /// 传给 mini-app-window 的参数
    pub fn args(&self) -> Vec<String> {
        match self {
            Drive::Drag { dy, frames } => vec!["--drag".into(), format!("{dy}x{frames}")],
            Drive::Frames(n) => vec!["--frames".into(), n.to_string()],
        }
    }
}
