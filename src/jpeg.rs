//! Baseline JPEG encoder (std only): YCbCr 4:2:0, the standard quantisation
//! and Huffman tables of the JPEG specification (Annex K).

const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20, 13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57,
    50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59, 52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

const LUM_Q: [u8; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56, 14, 17, 22, 29, 51, 87, 80, 62, 18, 22,
    37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113, 92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
];

const CHR_Q: [u8; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99, 99, 47, 66, 99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
];

const DC_LUM_BITS: [u8; 16] = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];
const DC_LUM_VAL: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const DC_CHR_BITS: [u8; 16] = [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0];
const DC_CHR_VAL: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const AC_LUM_BITS: [u8; 16] = [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7d];
const AC_LUM_VAL: [u8; 162] = [
    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91,
    0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a,
    0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53,
    0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79,
    0x7a, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5,
    0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9,
    0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf1, 0xf2,
    0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa,
];
const AC_CHR_BITS: [u8; 16] = [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77];
const AC_CHR_VAL: [u8; 162] = [
    0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71, 0x13, 0x22, 0x32, 0x81, 0x08, 0x14,
    0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0, 0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17,
    0x18, 0x19, 0x1a, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a,
    0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78,
    0x79, 0x7a, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3,
    0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7,
    0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf2,
    0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa,
];

/// Huffman code table: (code, length) per symbol.
fn build_huff(bits: &[u8; 16], vals: &[u8]) -> [(u16, u8); 256] {
    let mut t = [(0u16, 0u8); 256];
    let mut code = 0u16;
    let mut k = 0;
    for (l, &n) in bits.iter().enumerate() {
        for _ in 0..n {
            t[vals[k] as usize] = (code, l as u8 + 1);
            code += 1;
            k += 1;
        }
        code <<= 1;
    }
    t
}

struct Bits {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Bits {
    fn put(&mut self, code: u32, len: u32) {
        self.acc = (self.acc << len) | (code & ((1 << len) - 1));
        self.n += len;
        while self.n >= 8 {
            let b = (self.acc >> (self.n - 8)) as u8;
            self.out.push(b);
            if b == 0xFF {
                self.out.push(0);
            }
            self.n -= 8;
        }
    }
    fn flush(&mut self) {
        if self.n > 0 {
            let pad = 8 - self.n;
            self.put((1 << pad) - 1, pad);
        }
        self.n = 0;
        self.acc = 0;
    }
}

fn fdct(b: &mut [f32; 64]) {
    // separable float DCT (AAN would be faster; images are small enough)
    let mut tmp = [0f32; 64];
    let c = |u: usize| if u == 0 { std::f32::consts::FRAC_1_SQRT_2 } else { 1.0 };
    let cos = |x: usize, u: usize| (((2 * x + 1) as f32 * u as f32 * std::f32::consts::PI) / 16.0).cos();
    for y in 0..8 {
        for u in 0..8 {
            let mut s = 0.0;
            for x in 0..8 {
                s += b[y * 8 + x] * cos(x, u);
            }
            tmp[y * 8 + u] = s * c(u) / 2.0;
        }
    }
    for u in 0..8 {
        for v in 0..8 {
            let mut s = 0.0;
            for y in 0..8 {
                s += tmp[y * 8 + u] * cos(y, v);
            }
            b[v * 8 + u] = s * c(v) / 2.0;
        }
    }
}

fn bit_len(v: i32) -> u32 {
    32 - v.unsigned_abs().leading_zeros()
}

#[allow(clippy::too_many_arguments)]
fn encode_block(bits: &mut Bits, blk: &mut [f32; 64], q: &[f32; 64], dc_prev: &mut i32, dc: &[(u16, u8); 256], ac: &[(u16, u8); 256]) {
    fdct(blk);
    let mut z = [0i32; 64];
    for i in 0..64 {
        z[i] = (blk[ZIGZAG[i]] / q[ZIGZAG[i]]).round() as i32;
    }
    let diff = z[0] - *dc_prev;
    *dc_prev = z[0];
    let n = bit_len(diff);
    let (c, l) = dc[n as usize];
    bits.put(c as u32, l as u32);
    if n > 0 {
        let v = if diff < 0 { diff - 1 } else { diff };
        bits.put(v as u32, n);
    }
    let mut run = 0;
    for &v in z.iter().skip(1) {
        if v == 0 {
            run += 1;
            continue;
        }
        while run > 15 {
            let (c, l) = ac[0xF0];
            bits.put(c as u32, l as u32);
            run -= 16;
        }
        let n = bit_len(v);
        let (c, l) = ac[(run << 4 | n) as usize];
        bits.put(c as u32, l as u32);
        let vv = if v < 0 { v - 1 } else { v };
        bits.put(vv as u32, n);
        run = 0;
    }
    if run > 0 {
        let (c, l) = ac[0];
        bits.put(c as u32, l as u32);
    }
}

/// Encodes an RGB image (`quality` 1..100).
pub fn encode(width: usize, height: usize, rgb: &[u8], quality: u8) -> Vec<u8> {
    let q = quality.clamp(1, 100) as i32;
    let scale = if q < 50 { 5000 / q } else { 200 - q * 2 };
    let mk = |t: &[u8; 64]| -> [u8; 64] {
        let mut o = [0u8; 64];
        for i in 0..64 {
            o[i] = ((t[i] as i32 * scale + 50) / 100).clamp(1, 255) as u8;
        }
        o
    };
    let (lq, cq) = (mk(&LUM_Q), mk(&CHR_Q));
    let mut out = vec![0xFF, 0xD8];
    // JFIF
    out.extend_from_slice(&[0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1, 0, 1, 0, 0]);
    // quantisation tables (zigzag order)
    for (id, t) in [(0u8, &lq), (1u8, &cq)] {
        out.extend_from_slice(&[0xFF, 0xDB, 0, 67, id]);
        for i in 0..64 {
            out.push(t[ZIGZAG[i]]);
        }
    }
    // frame
    out.extend_from_slice(&[0xFF, 0xC0, 0, 17, 8, (height >> 8) as u8, height as u8, (width >> 8) as u8, width as u8, 3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
    // Huffman tables
    for (class, bits, vals) in [
        (0x00u8, &DC_LUM_BITS, &DC_LUM_VAL[..]),
        (0x10, &AC_LUM_BITS, &AC_LUM_VAL[..]),
        (0x01, &DC_CHR_BITS, &DC_CHR_VAL[..]),
        (0x11, &AC_CHR_BITS, &AC_CHR_VAL[..]),
    ] {
        let len = 3 + 16 + vals.len();
        out.extend_from_slice(&[0xFF, 0xC4, (len >> 8) as u8, len as u8, class]);
        out.extend_from_slice(bits);
        out.extend_from_slice(vals);
    }
    out.extend_from_slice(&[0xFF, 0xDA, 0, 12, 3, 1, 0x00, 2, 0x11, 3, 0x11, 0, 63, 0]);
    let (dcl, acl) = (build_huff(&DC_LUM_BITS, &DC_LUM_VAL), build_huff(&AC_LUM_BITS, &AC_LUM_VAL));
    let (dcc, acc) = (build_huff(&DC_CHR_BITS, &DC_CHR_VAL), build_huff(&AC_CHR_BITS, &AC_CHR_VAL));
    let lqf: [f32; 64] = std::array::from_fn(|i| lq[i] as f32);
    let cqf: [f32; 64] = std::array::from_fn(|i| cq[i] as f32);
    let px = |x: usize, y: usize| {
        let (x, y) = (x.min(width.saturating_sub(1)), y.min(height.saturating_sub(1)));
        let o = (y * width + x) * 3;
        let (r, g, b) = (rgb[o] as f32, rgb[o + 1] as f32, rgb[o + 2] as f32);
        (0.299 * r + 0.587 * g + 0.114 * b, -0.168736 * r - 0.331264 * g + 0.5 * b, 0.5 * r - 0.418688 * g - 0.081312 * b)
    };
    let mut bits = Bits { out: Vec::new(), acc: 0, n: 0 };
    let (mut py, mut pcb, mut pcr) = (0, 0, 0);
    for my in (0..height).step_by(16) {
        for mx in (0..width).step_by(16) {
            for by in 0..2 {
                for bx in 0..2 {
                    let mut blk = [0f32; 64];
                    for y in 0..8 {
                        for x in 0..8 {
                            blk[y * 8 + x] = px(mx + bx * 8 + x, my + by * 8 + y).0 - 128.0;
                        }
                    }
                    encode_block(&mut bits, &mut blk, &lqf, &mut py, &dcl, &acl);
                }
            }
            let mut cb = [0f32; 64];
            let mut cr = [0f32; 64];
            for y in 0..8 {
                for x in 0..8 {
                    let mut s = (0.0, 0.0);
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let p = px(mx + x * 2 + dx, my + y * 2 + dy);
                        s.0 += p.1;
                        s.1 += p.2;
                    }
                    cb[y * 8 + x] = s.0 / 4.0;
                    cr[y * 8 + x] = s.1 / 4.0;
                }
            }
            encode_block(&mut bits, &mut cb, &cqf, &mut pcb, &dcc, &acc);
            encode_block(&mut bits, &mut cr, &cqf, &mut pcr, &dcc, &acc);
        }
    }
    bits.flush();
    out.extend_from_slice(&bits.out);
    out.extend_from_slice(&[0xFF, 0xD9]);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn decodes_with_our_decoder() {
        let (w, h) = (37, 21);
        let mut rgb = vec![0u8; w * h * 3];
        for y in 0..h {
            for x in 0..w {
                let o = (y * w + x) * 3;
                rgb[o] = (x * 6) as u8;
                rgb[o + 1] = (y * 10) as u8;
                rgb[o + 2] = 128;
            }
        }
        let j = super::encode(w, h, &rgb, 95);
        let img = crate::image::decode(&j).unwrap();
        assert_eq!((img.width, img.height), (w, h));
        let p = img.pixel(20, 10);
        assert!((p[0] as i32 - 120).abs() < 12 && (p[1] as i32 - 100).abs() < 12 && (p[2] as i32 - 128).abs() < 12, "{p:?}");
    }
}
