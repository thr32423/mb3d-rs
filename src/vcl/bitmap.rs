//! Bitmaps of the toolkit: 32 bit ARGB pixels, decoded from the binary
//! properties of form files (`Glyph.Data`, `Picture.Data`) or set at run time.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bitmap {
    pub w: usize,
    pub h: usize,
    /// 0xAARRGGBB, rows top down
    pub px: Vec<u32>,
}

impl Bitmap {
    pub fn new(w: usize, h: usize, fill: u32) -> Bitmap {
        Bitmap { w, h, px: vec![fill; w * h] }
    }

    pub fn from_rgb(w: usize, h: usize, rgb: &[u8]) -> Bitmap {
        let px = rgb.chunks_exact(3).take(w * h).map(|c| 0xFF00_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32).collect();
        Bitmap { w, h, px }
    }

    pub fn is_empty(&self) -> bool {
        self.w == 0 || self.h == 0
    }

    pub fn get(&self, x: usize, y: usize) -> u32 {
        self.px[y * self.w + x]
    }

    /// Part of the bitmap (used for glyphs with several images side by side).
    pub fn crop(&self, x0: usize, y0: usize, w: usize, h: usize) -> Bitmap {
        let mut b = Bitmap::new(w, h, 0);
        for y in 0..h.min(self.h.saturating_sub(y0)) {
            for x in 0..w.min(self.w.saturating_sub(x0)) {
                b.px[y * w + x] = self.get(x0 + x, y0 + y);
            }
        }
        b
    }

    /// VCL glyph transparency: the colour of the bottom left pixel is
    /// transparent.
    pub fn with_transparent_corner(mut self) -> Bitmap {
        if self.is_empty() {
            return self;
        }
        let key = self.px[(self.h - 1) * self.w] & 0xFF_FFFF;
        for p in self.px.iter_mut() {
            if *p & 0xFF_FFFF == key {
                *p = 0;
            }
        }
        self
    }

    /// The disabled look of a glyph: embossed grey shape.
    pub fn disabled(&self, light: u32, dark: u32) -> Bitmap {
        let mut b = Bitmap::new(self.w, self.h, 0);
        for y in 0..self.h {
            for x in 0..self.w {
                let p = self.get(x, y);
                if p >> 24 < 128 {
                    continue;
                }
                let l = ((p >> 16 & 255) * 30 + (p >> 8 & 255) * 59 + (p & 255) * 11) / 100;
                if l < 192 {
                    b.px[y * self.w + x] = dark;
                    if x + 1 < self.w && y + 1 < self.h && b.px[(y + 1) * self.w + x + 1] == 0 {
                        b.px[(y + 1) * self.w + x + 1] = light;
                    }
                }
            }
        }
        b
    }
}

fn u16le(d: &[u8], o: usize) -> usize {
    d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize).unwrap_or(0)
}
fn u32le(d: &[u8], o: usize) -> u32 {
    d.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).unwrap_or(0)
}

/// Decodes a Windows BMP file (1, 4, 8, 16, 24 and 32 bit, RLE4/RLE8).
pub fn decode_bmp(d: &[u8]) -> Option<Bitmap> {
    if d.len() < 26 || &d[0..2] != b"BM" {
        return None;
    }
    let off = u32le(d, 10) as usize;
    let hs = u32le(d, 14) as usize;
    let (w, hraw, bpp, comp, ncol) = if hs == 12 {
        (u16le(d, 18) as i64, u16le(d, 20) as i16 as i64, u16le(d, 24), 0, 0)
    } else {
        (u32le(d, 18) as i32 as i64, u32le(d, 22) as i32 as i64, u16le(d, 28), u32le(d, 30), u32le(d, 46) as usize)
    };
    if w <= 0 || hraw == 0 || w > 1 << 15 || hraw.abs() > 1 << 15 {
        return None;
    }
    let (w, h) = (w as usize, hraw.unsigned_abs() as usize);
    let bottom_up = hraw > 0;
    let pal_entries = if bpp <= 8 { if ncol == 0 { 1 << bpp } else { ncol } } else { 0 };
    let pal_off = 14 + hs;
    let psz = if hs == 12 { 3 } else { 4 };
    let pal: Vec<u32> = (0..pal_entries)
        .map(|i| {
            let o = pal_off + i * psz;
            if o + 3 > d.len() {
                return 0xFF00_0000;
            }
            0xFF00_0000 | (d[o + 2] as u32) << 16 | (d[o + 1] as u32) << 8 | d[o] as u32
        })
        .collect();
    let mut b = Bitmap::new(w, h, 0xFF00_0000);
    let row_of = |y: usize| if bottom_up { h - 1 - y } else { y };
    if comp == 1 || comp == 2 {
        // RLE8 / RLE4
        let (mut x, mut y, mut i) = (0usize, 0usize, off);
        let set = |b: &mut Bitmap, x: usize, y: usize, c: usize| {
            if x < w && y < h {
                b.px[row_of(y) * w + x] = *pal.get(c).unwrap_or(&0xFF00_0000);
            }
        };
        while i + 1 < d.len() {
            let (n, c) = (d[i] as usize, d[i + 1] as usize);
            i += 2;
            if n > 0 {
                for k in 0..n {
                    let ci = if comp == 1 { c } else if k % 2 == 0 { c >> 4 } else { c & 15 };
                    set(&mut b, x, y, ci);
                    x += 1;
                }
            } else {
                match c {
                    0 => {
                        x = 0;
                        y += 1
                    }
                    1 => break,
                    2 => {
                        x += *d.get(i)? as usize;
                        y += *d.get(i + 1)? as usize;
                        i += 2
                    }
                    n => {
                        let bytes = if comp == 1 { n } else { n.div_ceil(2) };
                        for k in 0..n {
                            let v = if comp == 1 { *d.get(i + k)? as usize } else {
                                let byte = *d.get(i + k / 2)? as usize;
                                if k % 2 == 0 { byte >> 4 } else { byte & 15 }
                            };
                            set(&mut b, x, y, v);
                            x += 1;
                        }
                        i += bytes.div_ceil(2) * 2;
                    }
                }
            }
        }
        return Some(b);
    }
    let stride = (w * bpp as usize).div_ceil(32) * 4;
    let masks = if comp == 3 && hs >= 40 { Some((u32le(d, 54), u32le(d, 58), u32le(d, 62))) } else { None };
    let has_alpha = bpp == 32 && {
        // 32 bit bitmaps of the IDE mostly carry no alpha (all zero)
        let mut any = false;
        for y in 0..h {
            let o = off + y * stride;
            for x in 0..w {
                if d.get(o + x * 4 + 3).copied().unwrap_or(0) != 0 {
                    any = true;
                }
            }
        }
        any
    };
    for y in 0..h {
        let o = off + y * stride;
        let r = row_of(y);
        for x in 0..w {
            let p = match bpp {
                1 => {
                    let byte = *d.get(o + x / 8)?;
                    pal.get(((byte >> (7 - x % 8)) & 1) as usize).copied().unwrap_or(0)
                }
                4 => {
                    let byte = *d.get(o + x / 2)?;
                    pal.get(if x % 2 == 0 { byte >> 4 } else { byte & 15 } as usize).copied().unwrap_or(0)
                }
                8 => pal.get(*d.get(o + x)? as usize).copied().unwrap_or(0),
                16 => {
                    let v = u16le(d, o + x * 2) as u32;
                    let (rm, gm, bm) = masks.unwrap_or((0x7C00, 0x3E0, 0x1F));
                    let ch = |m: u32| {
                        if m == 0 {
                            return 0;
                        }
                        let sh = m.trailing_zeros();
                        let bits = (m >> sh).count_ones();
                        ((v & m) >> sh) * 255 / ((1 << bits) - 1)
                    };
                    0xFF00_0000 | ch(rm) << 16 | ch(gm) << 8 | ch(bm)
                }
                24 => {
                    let q = d.get(o + x * 3..o + x * 3 + 3)?;
                    0xFF00_0000 | (q[2] as u32) << 16 | (q[1] as u32) << 8 | q[0] as u32
                }
                32 => {
                    let q = d.get(o + x * 4..o + x * 4 + 4)?;
                    let a = if has_alpha { q[3] as u32 } else { 255 };
                    a << 24 | (q[2] as u32) << 16 | (q[1] as u32) << 8 | q[0] as u32
                }
                _ => return None,
            };
            b.px[r * w + x] = p;
        }
    }
    Some(b)
}

/// Decodes any picture format the program can read (BMP, PNG, JPEG).
pub fn decode_any(d: &[u8]) -> Option<Bitmap> {
    if d.starts_with(b"BM") {
        return decode_bmp(d);
    }
    let img = crate::image::decode(d).ok()?;
    let sh = if img.deep { 8 } else { 0 };
    let px = img.data.iter().map(|p| 0xFF00_0000 | ((p[0] >> sh) as u32) << 16 | ((p[1] >> sh) as u32) << 8 | (p[2] >> sh) as u32).collect();
    Some(Bitmap { w: img.width, h: img.height, px })
}

/// `Glyph.Data` / `Bitmap.Data`: a TBitmap stream (4 byte size + BMP file).
pub fn decode_tbitmap(d: &[u8]) -> Option<Bitmap> {
    if d.len() > 4 && &d[4..6] == b"BM" {
        return decode_bmp(&d[4..]);
    }
    decode_any(d)
}

/// `Picture.Data`: the graphic class name (short string) and its stream.
pub fn decode_picture(d: &[u8]) -> Option<Bitmap> {
    let n = *d.first()? as usize;
    let class = std::str::from_utf8(d.get(1..1 + n)?).ok()?;
    let rest = &d[1 + n..];
    if class.eq_ignore_ascii_case("TBitmap") {
        return decode_tbitmap(rest);
    }
    // TPngImage, TJPEGImage, ...: find the start of a known format
    for i in 0..rest.len().min(16) {
        let r = &rest[i..];
        if r.starts_with(&[0x89, b'P', b'N', b'G']) || r.starts_with(&[0xFF, 0xD8]) || r.starts_with(b"BM") {
            return decode_any(r);
        }
    }
    None
}

/// Images of a `TImageList` (`Bitmap = { 494C... }`): header followed by a
/// BMP strip of the images (and a mask bitmap).
pub fn decode_imagelist(d: &[u8]) -> Vec<Bitmap> {
    if d.len() < 28 || &d[0..2] != b"IL" {
        return Vec::new();
    }
    let count = u16le(d, 4);
    let (cx, cy) = (u16le(d, 8), u16le(d, 10));
    let Some(pos) = d.windows(2).position(|w| w == b"BM") else { return Vec::new() };
    let Some(strip) = decode_bmp(&d[pos..]) else { return Vec::new() };
    let per_row = (strip.w / cx.max(1)).max(1);
    (0..count).map(|i| strip.crop((i % per_row) * cx, (i / per_row) * cy, cx, cy)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_form_glyphs_decode() {
        for (name, text) in crate::app::forms::ALL {
            let o = crate::vcl::dfm::parse(text).unwrap();
            fn walk(o: &crate::vcl::dfm::Obj, name: &str) {
                for (k, v) in &o.props {
                    if let crate::vcl::dfm::Value::Bin(d) = v {
                        let ok = match k.as_str() {
                            "Glyph.Data" => decode_tbitmap(d).is_some(),
                            "Picture.Data" => decode_picture(d).is_some(),
                            _ => true,
                        };
                        assert!(ok, "{name}: {}.{k}", o.name);
                    }
                }
                for c in &o.children {
                    walk(c, name);
                }
            }
            walk(&o, name);
        }
    }
}
