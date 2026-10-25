//! 悦听音乐（sample/music）：每页能打开；歌单点歌进播放页、播放推进、进度条拖动 + 歌词 scroll-into-view、
//! 播放模式循环与上一首/下一首、单曲循环播完重来、收藏与最近播放持久化、迷你播放条（fixed）与队列弹层不穿透。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use serde_json::Value;

/// 点自定义 tabBar 的第 `i` 个（共 3 个）。选择器量得到就点节点中心，量不到按 tabBar 几何（底部 100rpx）点
fn tab(a: &mut App, i: usize) {
    let r = a.rects(".tab-item");
    if r.len() == 3 {
        let (x, y, w, h) = r[i];
        a.tap_xy(x + w / 2.0, y + h / 2.0);
    } else {
        a.tap_xy(W * (i as f32 + 0.5) / 3.0, H - 25.0);
    }
    a.settle();
}

/// 点第 `i` 个 `sel` 前先把它滚到 tabBar 与迷你条上方（底部 170 以内会被 fixed 的条盖住）
fn tap_visible(a: &mut App, sel: &str, i: usize) {
    for _ in 0..8 {
        let (_, y, _, h) = a.rects(sel)[i];
        let cy = y + h / 2.0;
        if cy < H - 170.0 {
            break;
        }
        a.scroll_by(W / 2.0, H / 2.0, (cy - H / 2.0).min(300.0));
    }
    a.tap_nth(sel, i);
}

fn num(v: &Value) -> i64 {
    v.as_i64().unwrap_or_else(|| panic!("不是整数：{v}"))
}

#[test]
fn 音乐_每页都能打开() {
    let _g = serial();
    smoke_all_pages("music", &[("pages/playlist/playlist", "id=p2"), ("pages/player/player", "id=5")]);
}

#[test]
fn 音乐_歌单点歌_进播放页_播放推进_记入最近播放() {
    let _g = serial();
    let mut a = App::open("music", None, "flow-play");
    assert_eq!(a.route(), "pages/discover/discover");
    assert_eq!(a.data("mini"), Value::Null, "还没放歌时不显示迷你条");
    assert!(!a.exists(".mini"));
    // 第 1 张歌单「夏日晚风」：[1, 4, 3, 16, 13, 2, 11]
    a.tap_settle(".pl");
    assert_eq!(a.route(), "pages/playlist/playlist");
    assert_eq!(a.data("pl.id"), "p1");
    assert_eq!(a.count(".pl-song"), 7, "wx:for 出 7 首");
    a.tap_nth(".pl-song", 2);
    a.settle();
    assert_eq!(a.route(), "pages/player/player");
    assert_eq!(a.depth(), 3);
    assert_eq!(a.data("song.id"), 3, "点的是第 3 首「纸船」");
    assert_eq!(a.data("song.name"), "纸船");
    assert_eq!(a.data("playing"), true);
    assert_eq!(a.data("queue").as_array().unwrap().len(), 7, "队列是整张歌单");
    let p0 = num(&a.data("pos"));
    a.wait_ms(2600);
    let p1 = num(&a.data("pos"));
    assert!(p1 >= p0 + 2, "setInterval 每秒推进 1 秒：{p0} → {p1}");
    assert_eq!(a.data("posText"), format!("00:{:02}", p1));
    // 暂停后不再推进
    a.tap(".play-btn");
    assert_eq!(a.data("playing"), false);
    let p2 = num(&a.data("pos"));
    a.wait_ms(1500);
    assert_eq!(num(&a.data("pos")), p2, "暂停后进度不动");
    assert_eq!(a.storage("mu_recent"), serde_json::json!([3]), "最近播放记下了这首");
    // 返回歌单页：当前曲目高亮、迷你条显示同一首
    a.back();
    assert_eq!(a.route(), "pages/playlist/playlist");
    assert_eq!(a.data("curId"), 3);
    assert_eq!(a.data("mini.name"), "纸船");
    assert!(a.exists(".mini"));
    a.assert_no_errors();
}

#[test]
fn 音乐_播放模式循环_上一首下一首() {
    let _g = serial();
    let mut a = App::open("music", Some("pages/playlist/playlist?id=p2"), "flow-mode");
    // 「深夜列车」：[5, 12, 15, 6, 7]
    a.tap_nth(".pl-song", 1);
    a.settle();
    assert_eq!(a.data("song.id"), 12);
    assert_eq!(a.data("modeText"), "顺序播放");
    a.tap(".next-btn");
    assert_eq!(a.data("song.id"), 15, "顺序：下一首");
    a.tap(".prev-btn");
    a.tap(".prev-btn");
    assert_eq!(a.data("song.id"), 5, "上一首两次回到第 1 首");
    a.tap(".prev-btn");
    assert_eq!(a.data("song.id"), 7, "第 1 首再上一首绕到最后一首");
    a.tap(".next-btn");
    assert_eq!(a.data("song.id"), 5, "最后一首的下一首绕回第 1 首");
    a.tap(".mode-btn");
    assert_eq!(a.data("modeText"), "单曲循环");
    assert_eq!(a.data("mode"), 1);
    a.tap(".mode-btn");
    assert_eq!(a.data("modeText"), "随机播放");
    let queue: Vec<i64> = a.data("queue").as_array().unwrap().iter().map(|s| num(&s["id"])).collect();
    for _ in 0..4 {
        let before = num(&a.data("song.id"));
        a.tap(".next-btn");
        let after = num(&a.data("song.id"));
        assert_ne!(before, after, "随机下一首换了一首");
        assert!(queue.contains(&after), "随机也只在队列里挑：{after}");
    }
    a.tap(".mode-btn");
    assert_eq!(a.data("modeText"), "顺序播放", "三种模式循环一圈");
    // 最近播放：最新的在前、不重复
    let recent = a.storage("mu_recent");
    let recent = recent.as_array().unwrap();
    assert_eq!(num(&recent[0]), num(&a.data("song.id")));
    let mut uniq: Vec<i64> = recent.iter().map(num).collect();
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), recent.len(), "最近播放去重");
    a.assert_no_errors();
}

#[test]
fn 音乐_拖进度条_歌词高亮并滚到当前行() {
    let _g = serial();
    let mut a = App::open("music", Some("pages/player/player?id=8"), "flow-seek");
    // 「雨后清晨」240 秒，每行歌词 6 秒
    assert_eq!(a.data("song.duration"), 240);
    a.tap(".play-btn"); // 先暂停，免得推进干扰
    assert_eq!(a.data("playing"), false);
    let (sx, sy, sw, sh) = a.rect(".slider");
    let cy = sy + sh / 2.0;
    // 滑块在最左（value 0）：轨道两端各让出半个滑块（block-size 14 → 7）
    let x0 = sx + 7.0;
    let span = sw - 14.0;
    a.drag(x0, cy, span * 0.5, 0.0, 10);
    a.frames(4);
    let pos = num(&a.data("pos"));
    assert!((108..=132).contains(&pos), "拖到一半左右：{pos}");
    let line = pos / 6;
    assert_eq!(num(&a.data("curLine")), line, "高亮行跟着进度");
    assert_eq!(a.data("toView"), format!("l{}", line - 2));
    assert_eq!(a.data("posText"), format!("{:02}:{:02}", pos / 60, pos % 60));
    a.wait_ms(700); // scroll-with-animation 滑过去
    let (_, ly, _, lh) = a.rect(".lyric");
    let (_, cy2, _, ch) = a.rect(&format!("#l{line}"));
    assert!(cy2 >= ly - 1.0 && cy2 + ch <= ly + lh + 1.0, "当前行滚进了歌词框：框 {ly}..{} 行 {cy2}", ly + lh);
    assert_eq!(a.count(".line.on"), 1, "只有一行高亮");
    // 点某一行歌词跳过去
    let (_, ly0, _, _) = a.rect(&format!("#l{}", line + 1));
    assert!(ly0 < ly + lh, "下一行也在框里");
    a.tap(&format!("#l{}", line + 1));
    assert_eq!(num(&a.data("pos")), (line + 1) * 6);
    assert_eq!(num(&a.data("curLine")), line + 1);
    a.assert_no_errors();
}

#[test]
fn 音乐_单曲循环播完重来_顺序播完切下一首() {
    let _g = serial();
    let mut a = App::open("music", Some("pages/player/player?id=3"), "flow-loop");
    a.tap(".mode-btn");
    assert_eq!(a.data("modeText"), "单曲循环");
    a.js("getCurrentPages().slice(-1)[0].onSeek({ detail: { value: 999 } })");
    a.frames(2);
    assert_eq!(num(&a.data("pos")), 182, "seek 夹到时长 - 1");
    a.wait_ms(2300);
    assert_eq!(a.data("song.id"), 3, "单曲循环：还是这首");
    assert!(num(&a.data("pos")) <= 2, "从头开始：{}", a.data("pos"));
    // 切回顺序（单曲 → 随机 → 顺序）
    a.tap(".mode-btn");
    a.tap(".mode-btn");
    assert_eq!(a.data("modeText"), "顺序播放");
    // 用真拖动拖到最右
    let (sx, sy, sw, sh) = a.rect(".slider");
    let p = num(&a.data("pos")) as f32 / 183.0;
    a.drag(sx + 7.0 + (sw - 14.0) * p, sy + sh / 2.0, sw, 0.0, 8);
    a.frames(2);
    assert!(num(&a.data("pos")) >= 181, "拖到底：{}", a.data("pos"));
    a.wait_ms(2300);
    assert_eq!(a.data("song.id"), 4, "全部单曲队列里 3 的下一首是 4");
    a.assert_no_errors();
}

#[test]
fn 音乐_收藏与最近播放_我的页显示_清空要确认() {
    let _g = serial();
    let mut a = App::open("music", None, "flow-fav");
    tap_visible(&mut a, ".daily", 1); // 每日推荐第 2 首：5「Midnight Train」
    a.settle();
    assert_eq!(a.route(), "pages/discover/discover", "点每日推荐只放歌不跳页");
    assert_eq!(a.data("mini.id"), 5);
    a.tap_settle(".mini-info");
    assert_eq!(a.route(), "pages/player/player");
    assert_eq!(a.data("fav"), false);
    a.tap(".fav");
    assert_eq!(a.data("fav"), true);
    assert_eq!(a.storage("mu_favs"), serde_json::json!([5]));
    a.tap(".next-btn"); // 每日推荐 [1, 5, 8, 11, 6, 13]：5 的下一首是 8，也记入最近播放
    assert_eq!(a.data("song.id"), 8);
    a.back();
    tab(&mut a, 2);
    assert_eq!(a.route(), "pages/mine/mine");
    assert_eq!(a.data("favCount"), 1);
    assert_eq!(a.data("list[0].name"), "Midnight Train");
    tab(&mut a, 0);
    tab(&mut a, 2);
    a.tap_nth(".tab", 1);
    assert_eq!(a.data("tab"), 1);
    assert_eq!(a.data("recentCount"), 2);
    assert_eq!(a.data("list[0].id"), 8, "最近播放最新的在前");
    a.tap(".clear");
    assert!(a.modal_open(), "清空前要确认");
    a.modal_tap(false);
    assert_eq!(a.data("recentCount"), 2, "取消不清");
    a.tap(".clear");
    a.modal_tap(true);
    assert_eq!(a.data("recentCount"), 0);
    assert_eq!(a.storage("mu_recent"), Value::String(String::new()), "storage 删掉了");
    // 「我的收藏」里点 ♥ 取消收藏
    a.tap_nth(".tab", 0);
    a.tap(".song-fav");
    assert_eq!(a.data("favCount"), 0);
    assert_eq!(a.storage("mu_favs"), serde_json::json!([]));
    a.assert_no_errors();
}

#[test]
fn 音乐_曲库筛选搜索_收藏按钮不触发播放() {
    let _g = serial();
    let mut a = App::open("music", Some("pages/library/library"), "flow-lib");
    assert_eq!(a.data("songs").as_array().unwrap().len(), 16);
    a.tap_nth(".chip", 2);
    assert_eq!(a.data("lang"), 2);
    let songs = a.data("songs");
    assert_eq!(songs.as_array().unwrap().len(), 5, "欧美 5 首");
    assert!(songs.as_array().unwrap().iter().all(|s| s["lang"] == "en"));
    a.type_into(".search-input", "Ivy");
    a.blur();
    assert_eq!(a.data("keyword"), "Ivy");
    assert_eq!(a.count(".lib-song"), 2, "Ivy Moon 两首");
    a.tap(".song-fav");
    assert_eq!(a.data("songs[0].fav"), true);
    assert_eq!(a.storage("mu_favs"), serde_json::json!([6]));
    assert_eq!(a.data("mini"), Value::Null, "catchtap 的收藏键不冒泡到整行的点歌");
    a.tap(".lib-song");
    a.frames(2);
    assert_eq!(a.data("curId"), 6);
    assert_eq!(a.data("mini.name"), "Golden Hour");
    a.type_into(".search-input", "不存在的歌");
    a.blur();
    assert_eq!(a.count(".lib-song"), 0);
    assert!(a.exists(".empty"), "无结果空态");
    a.tap(".search-clear");
    assert_eq!(a.count(".lib-song"), 5, "清除关键词回到欧美全部");
    a.assert_no_errors();
}

#[test]
fn 音乐_迷你条盖在列表上_点它不穿透_播放键不冒泡() {
    let _g = serial();
    let mut a = App::open("music", Some("pages/playlist/playlist?id=p6"), "flow-mini");
    a.tap_settle(".play-all");
    assert_eq!(a.route(), "pages/player/player");
    a.back();
    assert_eq!(a.data("mini.id"), 1);
    let (mx, my, mw, mh) = a.rect(".mini");
    assert!(my + mh <= H + 0.5 && my > H - 120.0, "迷你条贴在底部：{my}");
    // 迷你条下面确实压着一首歌（16 首的长列表）
    let rows = a.rects(".pl-song");
    let under = rows.iter().position(|&(_, y, _, h)| y < my + mh / 2.0 && y + h > my + mh / 2.0);
    assert!(under.is_some(), "迷你条中线下压着一行歌");
    a.tap_xy(mx + mw * 0.4, my + mh / 2.0);
    a.settle();
    assert_eq!(a.route(), "pages/player/player", "点迷你条进播放页");
    assert_eq!(a.data("song.id"), 1, "没有点到下面那首歌");
    a.back();
    a.tap(".mini-btn");
    assert_eq!(a.route(), "pages/playlist/playlist", "播放键 catchtap，不进播放页");
    assert_eq!(a.data("miniPlaying"), false);
    assert_eq!(a.data("mini.id"), 1);
    a.tap(".mini-btn");
    assert_eq!(a.data("miniPlaying"), true);
    a.assert_no_errors();
}

#[test]
fn 音乐_队列弹层开着_点下面的按钮不穿透() {
    let _g = serial();
    let mut a = App::open("music", Some("pages/playlist/playlist?id=p3"), "flow-queue");
    // 「专注白噪音」：[8, 9, 10, 14]
    a.tap_settle(".pl-song");
    assert_eq!(a.data("song.id"), 8);
    let (nx, ny, nw, nh) = a.rect(".fav");
    a.tap(".queue-btn");
    assert_eq!(a.data("showQueue"), true);
    assert_eq!(a.count(".q-item"), 4);
    // 弹层面板之外是遮罩：点收藏键所在的位置只关弹层
    let (_, sheet_y, _, _) = a.rect(".sheet");
    assert!(ny + nh / 2.0 < sheet_y, "收藏键在面板上方、被遮罩盖着");
    a.tap_xy(nx + nw / 2.0, ny + nh / 2.0);
    assert_eq!(a.data("showQueue"), false, "点遮罩关闭");
    assert_eq!(a.data("fav"), false, "没穿透到收藏键");
    assert_eq!(a.storage("mu_favs"), Value::String(String::new()));
    assert_eq!(a.data("song.id"), 8);
    a.tap(".queue-btn");
    a.tap(".sheet-title");
    assert_eq!(a.data("showQueue"), true, "点面板本身不关");
    a.tap_nth(".q-item", 3);
    assert_eq!(a.data("showQueue"), false);
    assert_eq!(a.data("song.id"), 14);
    assert_eq!(a.data("curIndex"), 3);
    a.assert_no_errors();
}
