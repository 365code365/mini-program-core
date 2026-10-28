//! 基 2 复数 FFT（就地、迭代）。只在引擎线程上用：振荡器波表的生成（osc.rs）与 AnalyserNode 的频谱（analyser.rs）。
//! 尺寸最大 32768（Analyser 的 fftSize 上限），没必要为它引一个依赖。

/// `re` / `im` 就地变换；`inverse` 时用 e^{+i…} 且**不**除以 N（调用方自己定标）
pub fn fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    assert!(n.is_power_of_two() && im.len() == n);
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut len = 2;
    while len <= n {
        let ang = sign * 2.0 * std::f64::consts::PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let (xr, xi) = (re[b] * cr - im[b] * ci, re[b] * ci + im[b] * cr);
                re[b] = re[a] - xr;
                im[b] = im[a] - xi;
                re[a] += xr;
                im[a] += xi;
                (cr, ci) = (cr * wr - ci * wi, cr * wi + ci * wr);
            }
        }
        len <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 与直接求和一致_正反变换还原() {
        let n = 64;
        let x: Vec<f64> = (0..n).map(|i| ((i * 7 % 13) as f64 - 6.0) / 6.0).collect();
        let (mut re, mut im) = (x.clone(), vec![0.0; n]);
        fft(&mut re, &mut im, false);
        for k in [0, 1, 5, 31] {
            let (mut sr, mut si) = (0.0, 0.0);
            for (i, v) in x.iter().enumerate() {
                let a = -2.0 * std::f64::consts::PI * (k * i) as f64 / n as f64;
                sr += v * a.cos();
                si += v * a.sin();
            }
            assert!((re[k] - sr).abs() < 1e-9 && (im[k] - si).abs() < 1e-9, "k={k}");
        }
        fft(&mut re, &mut im, true);
        for i in 0..n {
            assert!((re[i] / n as f64 - x[i]).abs() < 1e-12);
        }
    }
}
