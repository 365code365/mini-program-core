//! ChannelMergerNode / ChannelSplitterNode（规范 §1.15 / §1.16）。
//! - Merger：n 路输入，每路 channelCount = 1、explicit、speakers（立体声进来先按 0.5(L+R) 下混成单声道，图在混输入时做），
//!   输出 n 声道，第 i 声道 = 第 i 路输入；
//! - Splitter：一路输入，channelCount = n、explicit、discrete；第 i 路输出是单声道的第 i 声道（没有就静音）。

use super::bus::Bus;
use super::{Flow, Node, Param, Quantum};

pub struct Merger;

impl Node for Merger {
    fn process(&mut self, _q: &Quantum, ins: &[Bus], _params: &[Param], outs: &mut [Bus]) -> Flow {
        let out = &mut outs[0];
        out.reset(ins.len());
        for (i, b) in ins.iter().enumerate().take(out.ch) {
            if b.ch > 0 {
                out.chan_mut(i).copy_from_slice(b.chan(0));
            }
        }
        Flow::Live
    }
}

pub struct Splitter;

impl Node for Splitter {
    fn process(&mut self, _q: &Quantum, ins: &[Bus], _params: &[Param], outs: &mut [Bus]) -> Flow {
        let inp = &ins[0];
        for (i, o) in outs.iter_mut().enumerate() {
            o.reset(1);
            if i < inp.ch {
                o.chan_mut(0).copy_from_slice(inp.chan(i));
            }
        }
        Flow::Live
    }
}
