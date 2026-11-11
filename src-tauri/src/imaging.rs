//! 位图处理（自定义形象导入域）：纯色底自动透明化。
//!
//! 用户导入的 AI 生成图常带白底（非透明 PNG），渲染时表现为白色方块。
//! 策略：仅对**基本不透明**的图做「边缘连通洪泛去底」——从四边取样背景色，
//! 把与图像边界连通的近似背景像素置透明；被角色轮廓包围的内部同色区域
//! （如白色毛发）不与边界连通，得以保留。命令层（`commands/custom_avatar.rs`）
//! 只调用本模块，算法内聚此处、可直测。
//!
//! 局限：依赖角色有可见轮廓把洪泛挡在内部同色区之外；无轮廓的白底白角色
//! 会被一并移除——该场景只能请用户提供真正的透明底素材。

use image::ExtendedColorType;
use image::ImageEncoder as _;
use image::codecs::png::PngEncoder;
use image::{RgbaImage};

/// 背景判定容差：与背景色 RGB 欧氏距离平方 ≤ 40²（每通道约 ±23）视为背景。
const BG_TOL2: i64 = 40 * 40;
/// 羽化上限倍数：移除区邻接像素与背景距离在 `FEATHER_MULT × BG_TOL2` 内时按比例半透明，
/// 弱化抗锯齿产生的白边（kept 像素必然 d > BG_TOL2，故 factor ∈ (0.5, 1)）。
const FEATHER_MULT: i64 = 2;

/// RGB 距离平方（不含 alpha）。
#[inline]
fn rgb_dist2(px: [u8; 4], bg: [i64; 3]) -> i64 {
    let dr = i64::from(px[0]) - bg[0];
    let dg = i64::from(px[1]) - bg[1];
    let db = i64::from(px[2]) - bg[2];
    dr * dr + dg * dg + db * db
}

/// 是否基本不透明（透明像素占比 < 1%）：已含透明的图不做去底，尊重作者已抠好的 alpha。
pub fn is_mostly_opaque(img: &RgbaImage) -> bool {
    let total = u64::from(img.width()) * u64::from(img.height());
    if total == 0 {
        return true;
    }
    let transparent = img.pixels().filter(|p| p.0[3] < 255).count() as u64;
    transparent * 100 < total
}

/// 边缘连通洪泛去底：四边取样背景色 → BFS 移除连通近似背景 → 邻接羽化。
///
/// 只处理基本不透明的图；已含透明的图原样克隆返回。
#[allow(clippy::indexing_slicing)] // removed/stack 索引均由 idx(x,y)=y*w+x ∈ [0, w*h) 构造保证
pub fn remove_flat_background(img: &RgbaImage) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 || !is_mostly_opaque(img) {
        return img.clone();
    }
    let idx = |x: u32, y: u32| (y * w + x) as usize;

    // 1) 背景色 = 四边像素 RGB 均值（AI 白底图四边同色，均值稳健；角点重复计入无碍）
    let (mut sr, mut sg, mut sb, mut n) = (0i64, 0i64, 0i64, 0u64);
    for x in 0..w {
        for &y in &[0, h - 1] {
            let p = img.get_pixel(x, y).0;
            sr += i64::from(p[0]);
            sg += i64::from(p[1]);
            sb += i64::from(p[2]);
            n += 1;
        }
    }
    for y in 0..h {
        for &x in &[0, w - 1] {
            let p = img.get_pixel(x, y).0;
            sr += i64::from(p[0]);
            sg += i64::from(p[1]);
            sb += i64::from(p[2]);
            n += 1;
        }
    }
    let bg = [sr / i64::try_from(n).unwrap_or(1), sg / i64::try_from(n).unwrap_or(1), sb / i64::try_from(n).unwrap_or(1)];

    // 2) 种子：四边上近似背景色的像素；BFS 沿近似背景色扩散（4 邻域）
    let mut removed = vec![false; (w * h) as usize];
    let mut stack: Vec<usize> = Vec::new();
    for x in 0..w {
        for &y in &[0, h - 1] {
            let i = idx(x, y);
            if !removed[i] && rgb_dist2(img.get_pixel(x, y).0, bg) <= BG_TOL2 {
                removed[i] = true;
                stack.push(i);
            }
        }
    }
    for y in 0..h {
        for &x in &[0, w - 1] {
            let i = idx(x, y);
            if !removed[i] && rgb_dist2(img.get_pixel(x, y).0, bg) <= BG_TOL2 {
                removed[i] = true;
                stack.push(i);
            }
        }
    }
    while let Some(i) = stack.pop() {
        let (x, y) = ((i as u32) % w, (i as u32) / w);
        for (nx, ny) in neighbors4(x, y, w, h) {
            let j = idx(nx, ny);
            if !removed[j] && rgb_dist2(img.get_pixel(nx, ny).0, bg) <= BG_TOL2 {
                removed[j] = true;
                stack.push(j);
            }
        }
    }

    let mut out = img.clone();
    for (i, r) in removed.iter().enumerate() {
        if *r {
            let (x, y) = ((i as u32) % w, (i as u32) / w);
            out.get_pixel_mut(x, y).0[3] = 0;
        }
    }

    // 3) 羽化：与移除区 4-邻接的保留像素按背景距离比例衰减 alpha（弱化白边）
    for y in 0..h {
        for x in 0..w {
            let i = idx(x, y);
            if removed[i] {
                continue;
            }
            let touches = neighbors4(x, y, w, h).iter().any(|&(nx, ny)| removed[idx(nx, ny)]);
            if !touches {
                continue;
            }
            let d = rgb_dist2(img.get_pixel(x, y).0, bg);
            let cap = BG_TOL2 * FEATHER_MULT;
            if d < cap {
                let p = out.get_pixel_mut(x, y);
                let factor = d as f64 / cap as f64;
                p.0[3] = (f64::from(p.0[3]) * factor).round() as u8;
            }
        }
    }
    out
}

/// 四邻域坐标（已裁到图内）。
fn neighbors4(x: u32, y: u32, w: u32, h: u32) -> Vec<(u32, u32)> {
    const DX: [i64; 4] = [-1, 1, 0, 0];
    const DY: [i64; 4] = [0, 0, -1, 1];
    let mut out = Vec::with_capacity(4);
    for (&dx, &dy) in DX.iter().zip(DY.iter()) {
        let nx = i64::from(x) + dx;
        let ny = i64::from(y) + dy;
        if nx >= 0 && ny >= 0 && nx < i64::from(w) && ny < i64::from(h) {
            out.push((u32::try_from(nx).unwrap_or(0), u32::try_from(ny).unwrap_or(0)));
        }
    }
    out
}

/// 导入管线入口：解码 PNG →（基本不透明时）边缘连通去底 → 编码回 PNG 字节。
///
/// 已含透明的图**原样返回**（不重编码）；解码失败即图片损坏，由调用方拒绝导入。
pub fn process_avatar_png(bytes: &[u8]) -> Result<Vec<u8>, image::ImageError> {
    let rgba = image::load_from_memory(bytes)?.to_rgba8();
    if !is_mostly_opaque(&rgba) {
        return Ok(bytes.to_vec());
    }
    let cleaned = remove_flat_background(&rgba);
    let mut out = Vec::new();
    PngEncoder::new(&mut out).write_image(
        cleaned.as_raw(),
        cleaned.width(),
        cleaned.height(),
        ExtendedColorType::Rgba8,
    )?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    /// 纯色画布。
    fn solid(w: u32, h: u32, px: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba(px))
    }

    #[test]
    fn is_mostly_opaque_detects_transparency_ratio() {
        let mut img = solid(20, 10, [255, 0, 0, 255]);
        assert!(is_mostly_opaque(&img));
        *img.get_pixel_mut(0, 0) = Rgba([0, 0, 0, 0]); // 0.5% 透明仍算基本不透明
        assert!(is_mostly_opaque(&img));
        for x in 1..6u32 {
            *img.get_pixel_mut(x, 1) = Rgba([0, 0, 0, 0]); // 共 3% 透明
        }
        assert!(!is_mostly_opaque(&img));
    }

    #[test]
    fn already_transparent_image_untouched() {
        let mut img = solid(10, 10, [255, 255, 255, 255]);
        *img.get_pixel_mut(0, 0) = Rgba([1, 2, 3, 0]);
        let out = remove_flat_background(&img);
        assert_eq!(out.as_raw(), img.as_raw(), "已含透明的图不得被改写");
    }

    #[test]
    fn white_background_removed_center_kept() {
        let mut img = solid(20, 20, [255, 255, 255, 255]);
        for y in 8..12u32 {
            for x in 8..12u32 {
                *img.get_pixel_mut(x, y) = Rgba([0, 0, 0, 255]);
            }
        }
        let out = remove_flat_background(&img);
        assert_eq!(out.get_pixel(1, 1).0[3], 0, "背景应被移除");
        assert_eq!(out.get_pixel(10, 10).0[3], 255, "主体应保留");
        assert_eq!(out.get_pixel(10, 10).0[0], 0, "主体颜色不变");
    }

    #[test]
    fn enclosed_white_region_survives() {
        // 白底 + 灰色轮廓环 + 环内白色（模拟白色毛发被轮廓包住）
        let mut img = solid(20, 20, [255, 255, 255, 255]);
        for k in 5..=14u32 {
            for (x, y) in [(k, 5u32), (k, 14u32), (5u32, k), (14u32, k)] {
                *img.get_pixel_mut(x, y) = Rgba([100, 100, 100, 255]);
            }
        }
        let out = remove_flat_background(&img);
        assert_eq!(out.get_pixel(1, 1).0[3], 0, "外部白底应被移除");
        assert_eq!(out.get_pixel(10, 10).0[3], 255, "轮廓内的白色应保留");
        assert_eq!(out.get_pixel(5, 10).0[3], 255, "轮廓本身应保留");
    }

    #[test]
    fn fringe_feathered_near_background() {
        // 白底 + 一条不触边的 (231,231,231) 竖条（触边会被计入背景色均值拉偏取样）：
        // d=3·24²=1728 ∈ (BG_TOL2, 2·BG_TOL2) → 保留但羽化
        let mut img = solid(20, 20, [255, 255, 255, 255]);
        for y in 4..16u32 {
            *img.get_pixel_mut(10, y) = Rgba([231, 231, 231, 255]);
        }
        let out = remove_flat_background(&img);
        assert_eq!(out.get_pixel(5, 5).0[3], 0, "背景移除");
        let a = out.get_pixel(10, 5).0[3];
        assert!(a > 0 && a < 255, "邻接羽化应半透明，实际 {a}");
        assert_eq!(out.get_pixel(10, 5).0[0], 231, "羽化不改 RGB");
        assert_eq!(out.get_pixel(10, 0).0[3], 0, "竖条外的顶部白底移除");
    }

    #[test]
    fn process_roundtrip_encodes_png() {
        // 8×8 白底图 → 编码 PNG → 管线处理 → 解码校验背景已透明
        let img = solid(8, 8, [255, 255, 255, 255]);
        let mut raw = Vec::new();
        PngEncoder::new(&mut raw)
            .write_image(img.as_raw(), 8, 8, ExtendedColorType::Rgba8)
            .expect("编码测试图");
        let out = process_avatar_png(&raw).expect("处理成功");
        let decoded = image::load_from_memory(&out).expect("回读").to_rgba8();
        assert_eq!(decoded.get_pixel(0, 0).0[3], 0, "处理后背景透明");
    }
}
