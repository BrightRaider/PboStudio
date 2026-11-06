//! The program's mark, drawn: an amber ring with a dot on a dark disc. The window icon, the icon of the executable in the
//! Explorer and the mark in the header are all this one picture. Nothing is embedded as an image file; this module uses
//! nothing but the standard library, so the build script can include it as it is.

/// The picture as straight-alpha RGBA, `size` by `size` pixels. Every pixel is the mean of 4 by 4 samples, so the edges
/// are smooth at any size.
pub fn rgba(size: usize) -> Vec<u8> {
    const SAMPLES: usize = 4;
    let half = size as f32 / 2.0;
    let unit = half / 32.0;
    let mut out = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let (mut r, mut g, mut b, mut a) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let px = x as f32 + (sx as f32 + 0.5) / SAMPLES as f32 - half;
                    let py = y as f32 + (sy as f32 + 0.5) / SAMPLES as f32 - half;
                    let d = (px * px + py * py).sqrt() / unit;
                    if d > 31.0 {
                        continue;
                    }
                    let amber = (17.0..=25.0).contains(&d) || d <= 8.0;
                    let (cr, cg, cb) = if amber {
                        (0xF5 as f32, 0xA6 as f32, 0x23 as f32)
                    } else {
                        (0x16 as f32, 0x17 as f32, 0x1A as f32)
                    };
                    r += cr;
                    g += cg;
                    b += cb;
                    a += 1.0;
                }
            }
            let n = (SAMPLES * SAMPLES) as f32;
            if a > 0.0 {
                out.extend_from_slice(&[
                    (r / a).round() as u8,
                    (g / a).round() as u8,
                    (b / a).round() as u8,
                    (a / n * 255.0).round() as u8,
                ]);
            } else {
                out.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::rgba;

    #[test]
    fn the_picture_has_the_size_asked_for_and_a_transparent_corner() {
        for size in [16usize, 32, 48, 64, 256] {
            let p = rgba(size);
            assert_eq!(p.len(), size * size * 4, "{size}");
            assert_eq!(p[3], 0, "the corner of {size} is empty");
            // The middle is the amber dot.
            let mid = ((size / 2) * size + size / 2) * 4;
            assert_eq!(&p[mid..mid + 4], &[0xF5, 0xA6, 0x23, 0xFF], "{size}");
        }
    }
}
