//! Fontdue is a font parser, rasterizer, and layout tool.
//!
//! This is a no_std crate, but still requires the alloc crate.

#![cfg_attr(all(not(test), not(feature = "std"), feature = "hashbrown"), no_std)]
#![allow(dead_code)]
// 上游 0.9.3 原样的代码在新 rustc 上的提示（从 crates.io 拉的时候被 cap-lints 压掉了，path 依赖不压）
#![allow(unnecessary_transmutes)]
#![allow(clippy::style)]
#![allow(clippy::complexity)]
#![allow(clippy::misnamed_getters)]

extern crate alloc;

mod font;
mod hash;
/// Tools for laying out strings of text.
pub mod layout;
mod math;
mod platform;
mod raster;
mod table;
mod unicode;

pub use crate::font::*;

#[cfg(feature = "hashbrown")]
pub(crate) use hashbrown::{HashMap, HashSet};
#[cfg(not(feature = "hashbrown"))]
pub(crate) use std::collections::{HashMap, HashSet};

/// Alias for Result<T, &'static str>.
pub type FontResult<T> = Result<T, &'static str>;
