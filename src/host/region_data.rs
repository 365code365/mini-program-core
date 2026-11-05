//! `<picker mode="region">` 用的省 / 市 / 区数据（带统计用区划代码）。
//!
//! **数据表例外（steering 20）**：数据本身在 `region_pca.txt`（31 个省级、342 个地级、3056 个县级，约 60KB），
//! 这里只有解析与查表。
//!
//! 出处：国家统计局《2023年统计用区划代码和城乡划分代码》（截止 2023-06-30，发布 2023-09-11）的公开整理版
//! github.com/modood/Administrative-divisions-of-China `dist/pca-code.json`（2026-11-05 取）。转成每行一个地级的紧凑文本：
//! `#<2 位省码><省名>` 起一个省；`<4 位市码><市名> <6 位县码><县名> …` 一行一个市。
//! 改动：北京 / 天津 / 上海 / 重庆的「市辖区」改名成直辖市名（微信的 region picker 显示「北京市 / 北京市 / 东城区」），
//! 其余名字原样（「省直辖县级行政区划」「县」等照统计局）。
//!
//! 与微信的已知差别（待真机核对）：
//! - 没有台湾省、香港、澳门（统计局表里没有）；
//! - 东莞 / 中山 / 儋州 / 嘉峪关不设区，第三列是统计局给的乡镇街道（9 位码）；
//! - `postcode`（邮政编码）没有可信的公开数据，不给；
//! - `level="sub-district"`（街道）没有数据，按 `region` 三列处理。
//!
//! 按需解析：第一次弹 region picker 时整表解析一次（约 3400 条，亚毫秒），之后常驻；不弹就不解析。
//! 代码按微信的 6 位给（省 `110000`、市 `110100`、县原样）。
use std::sync::OnceLock;

static RAW: &str = include_str!("region_pca.txt");

pub struct City {
    pub code: String,
    pub name: String,
    /// (6 位县码，县名)
    pub districts: Vec<(String, String)>,
}

pub struct Province {
    pub code: String,
    pub name: String,
    pub cities: Vec<City>,
}

/// 名字前面的数字是代码
fn split_code(s: &str) -> (&str, &str) {
    let i = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    (&s[..i], &s[i..])
}

/// 补到 6 位（省 2 位 → `xx0000`，市 4 位 → `xxxx00`）
fn six(code: &str) -> String {
    if code.len() >= 6 { code.to_string() } else { format!("{code:0<6}") }
}

fn parse() -> Vec<Province> {
    let mut out: Vec<Province> = Vec::with_capacity(34);
    for line in RAW.lines() {
        if let Some(p) = line.strip_prefix('#') {
            let (code, name) = split_code(p);
            out.push(Province { code: six(code), name: name.to_string(), cities: Vec::new() });
            continue;
        }
        let mut parts = line.split(' ').filter(|s| !s.is_empty());
        let Some(head) = parts.next() else { continue };
        let Some(prov) = out.last_mut() else { continue };
        let (code, name) = split_code(head);
        let districts = parts.map(|d| split_code(d)).map(|(c, n)| (six(c), n.to_string())).collect();
        prov.cities.push(City { code: six(code), name: name.to_string(), districts });
    }
    out
}

/// 全表（第一次调用时解析）
pub fn provinces() -> &'static [Province] {
    static TABLE: OnceLock<Vec<Province>> = OnceLock::new();
    TABLE.get_or_init(parse)
}

/// 第 pi 个省的市名列表
pub fn cities_of(pi: usize) -> Vec<String> {
    provinces().get(pi).map(|p| p.cities.iter().map(|c| c.name.clone()).collect()).unwrap_or_default()
}

/// 第 pi 个省、第 ci 个市的区名列表
pub fn districts_of(pi: usize, ci: usize) -> Vec<String> {
    provinces()
        .get(pi)
        .and_then(|p| p.cities.get(ci))
        .map(|c| c.districts.iter().map(|d| d.1.clone()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 全表_直辖市改名_六位码() {
        let p = provinces();
        assert_eq!(p.len(), 31);
        assert_eq!(p.iter().map(|p| p.cities.len()).sum::<usize>(), 342);
        assert_eq!(p[0].name, "北京市");
        assert_eq!(p[0].code, "110000");
        assert_eq!(p[0].cities[0].name, "北京市", "直辖市的「市辖区」显示成直辖市名");
        assert_eq!(p[0].cities[0].code, "110100");
        assert_eq!(p[0].cities[0].districts[0], ("110101".to_string(), "东城区".to_string()));
        let gd = p.iter().find(|p| p.name == "广东省").unwrap();
        let gz = gd.cities.iter().find(|c| c.name == "广州市").unwrap();
        assert!(gz.districts.iter().any(|d| d.1 == "海珠区" && d.0 == "440105"));
    }
}
