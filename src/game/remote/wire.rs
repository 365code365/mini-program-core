//! 线格式的读（页面 → 原生，见 mod.rs 文件头）与返回值的编码（原生 → 页面，JSON）。

use rquickjs::{Ctx, Function, TypedArray, Value};

pub fn head(b: &[u8], pos: &mut usize) -> Option<(u8, u8, usize)> {
    let h = b.get(*pos..*pos + 4)?;
    *pos += 4;
    Some((h[0], h[1], u16::from_le_bytes([h[2], h[3]]) as usize))
}

fn u32_at(b: &[u8], pos: &mut usize) -> Option<u32> {
    let v = u32::from_le_bytes(b.get(*pos..*pos + 4)?.try_into().ok()?);
    *pos += 4;
    Some(v)
}

fn bytes<'a>(b: &'a [u8], pos: &mut usize) -> Option<&'a [u8]> {
    let n = u32_at(b, pos)? as usize;
    let s = b.get(*pos..*pos + n)?;
    *pos += n;
    Some(s)
}

/// 按元素类型把字节拷成对应的类型化数组（对齐不要求：逐元素读）
macro_rules! typed {
    ($ctx:expr, $raw:expr, $t:ty) => {{
        const SZ: usize = std::mem::size_of::<$t>();
        let v: Vec<$t> = $raw.chunks_exact(SZ).map(|x| <$t>::from_le_bytes(x.try_into().unwrap())).collect();
        TypedArray::<$t>::new($ctx.clone(), v).ok().map(|a| a.into_value())?
    }};
}

pub fn read<'js>(ctx: &Ctx<'js>, b: &[u8], pos: &mut usize) -> Option<Value<'js>> {
    let tag = *b.get(*pos)?;
    *pos += 1;
    let c = || ctx.clone();
    Some(match tag {
        0 => Value::new_undefined(c()),
        1 => Value::new_null(c()),
        2 => Value::new_bool(c(), false),
        3 => Value::new_bool(c(), true),
        4 => {
            let v = f64::from_le_bytes(b.get(*pos..*pos + 8)?.try_into().ok()?);
            *pos += 8;
            Value::new_number(c(), v)
        }
        5 => {
            let v = i32::from_le_bytes(b.get(*pos..*pos + 4)?.try_into().ok()?);
            *pos += 4;
            Value::new_int(c(), v)
        }
        6 => {
            let s = std::str::from_utf8(bytes(b, pos)?).ok()?;
            rquickjs::String::from_str(c(), s).ok()?.into_value()
        }
        7 => {
            let kind = *b.get(*pos)?;
            *pos += 1;
            let raw = bytes(b, pos)?;
            match kind {
                1 => typed!(ctx, raw, i8),
                2 | 3 => typed!(ctx, raw, u8),
                4 => typed!(ctx, raw, i16),
                5 => typed!(ctx, raw, u16),
                6 => typed!(ctx, raw, i32),
                7 => typed!(ctx, raw, u32),
                8 => typed!(ctx, raw, f32),
                9 => typed!(ctx, raw, f64),
                _ => rquickjs::ArrayBuffer::new(c(), raw.to_vec()).ok()?.into_value(),
            }
        }
        8 => {
            let n = u32_at(b, pos)? as usize;
            let arr = rquickjs::Array::new(c()).ok()?;
            for i in 0..n {
                let v = read(ctx, b, pos)?;
                arr.set(i, v).ok()?;
            }
            arr.into_value()
        }
        9 => {
            let s = std::str::from_utf8(bytes(b, pos)?).ok()?;
            ctx.json_parse(s).ok()?
        }
        _ => return None,
    })
}

/// 返回值 → JSON 串（类型化数组、ArrayBuffer、undefined 带标记，init.js 的 `__wgrEnc`）
pub fn encode<'js>(ctx: &Ctx<'js>, v: Value<'js>) -> String {
    ctx.globals()
        .get::<_, Function>("__wgrEnc")
        .and_then(|f| f.call::<_, String>((v,)))
        .unwrap_or_else(|_| r#"{"$u":1}"#.into())
}
