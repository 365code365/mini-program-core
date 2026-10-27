//! 处理计划：一个上下文里还有槽位的节点按拓扑序排好（上游先算），每个节点带上自己的入边。
//! 不在 destination 上游的节点也算（Analyser 没接 destination 也要有数据、没接上的源也要按时播完报 ended）。
//! 环：Kahn 排完剩下的节点按建立顺序接在后面，环上的边读到的是上一个量子的输出（graph/mod.rs 文件头）。

use super::super::graph::{Edge, Plan};
use super::WebAudio;
use std::collections::HashMap;

pub fn build(w: &WebAudio, ctx: usize) -> Option<Plan> {
    let c = w.ctxs.get(&ctx)?;
    let live: Vec<(u32, u32)> = c.nodes.iter().filter_map(|id| w.nodes.get(id).and_then(|n| n.slot).map(|s| (*id, s))).collect();
    let slot_of: HashMap<u32, u32> = live.iter().copied().collect();
    let mut edges: Vec<Edge> = Vec::new();
    for &(id, src) in &live {
        for cn in &w.nodes[&id].outs {
            if let Some(&dst) = slot_of.get(&cn.to) {
                edges.push(Edge { dst, port: cn.port, param: cn.param, src, out: cn.out });
            }
        }
    }
    // Kahn：入度按「不同的上游节点」算，自环不算
    let mut indeg: HashMap<u32, usize> = live.iter().map(|&(_, s)| (s, 0)).collect();
    let mut succ: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    for e in &edges {
        if e.src != e.dst && seen.insert((e.src, e.dst)) {
            *indeg.get_mut(&e.dst).unwrap() += 1;
            succ.entry(e.src).or_default().push(e.dst);
        }
    }
    let mut order = Vec::with_capacity(live.len());
    let mut ready: Vec<u32> = live.iter().map(|&(_, s)| s).filter(|s| indeg[s] == 0).collect();
    ready.reverse();
    while let Some(s) = ready.pop() {
        order.push(s);
        for &d in succ.get(&s).map(Vec::as_slice).unwrap_or(&[]) {
            let k = indeg.get_mut(&d).unwrap();
            *k -= 1;
            if *k == 0 {
                ready.push(d);
            }
        }
    }
    for &(_, s) in &live {
        if !order.contains(&s) {
            order.push(s);
        }
    }
    let mut p = Plan { dest: slot_of.get(&c.dest).copied().unwrap_or(u32::MAX), ..Default::default() };
    for &s in &order {
        p.starts.push(p.edges.len() as u32);
        p.order.push(s);
        p.edges.extend(edges.iter().filter(|e| e.dst == s));
    }
    p.starts.push(p.edges.len() as u32);
    Some(p)
}
