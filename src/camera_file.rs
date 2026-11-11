//! The Position panel's values (.m3xyz) and the view rotation (.m3r) in
//! small text files of their own, to bring a view to another scene. Not in
//! MB3D; the idea is Geoffrey N. Barnes's.
//!
//! The rotation is kept as the matrix: at a y angle of +-90 degrees the
//! Euler angles do not exist (the panel shows "?"), and angles rounded to
//! text would move a deep zoom's view.

use crate::math::Mat3;

const POSITION_HEADER: &str = "Mandelbulb3D position";
const ROTATION_HEADER: &str = "Mandelbulb3D rotation";
/// Far more than a real file; a wrong file chosen by mistake is not read.
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub mid: [f64; 3],
    pub z_start: f64,
    pub z_end: f64,
    pub zoom: f64,
}

/// `{:?}` of an f64 is the shortest text that reads back as the same bits.
pub fn position_text(p: &Position) -> String {
    format!(
        "{POSITION_HEADER}\nx_mid = {:?}\ny_mid = {:?}\nz_mid = {:?}\nz_start = {:?}\nz_end = {:?}\nzoom = {:?}\n",
        p.mid[0], p.mid[1], p.mid[2], p.z_start, p.z_end, p.zoom
    )
}

pub fn rotation_text(m: &Mat3) -> String {
    let row = |r: &[f64; 3]| format!("{:?} {:?} {:?}", r[0], r[1], r[2]);
    format!("{ROTATION_HEADER}\nrow1 = {}\nrow2 = {}\nrow3 = {}\n", row(&m[0]), row(&m[1]), row(&m[2]))
}

/// The text after `key =` on a line of `text`.
fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == key).then(|| v.trim())
    })
}

fn number(s: &str) -> Option<f64> {
    s.parse::<f64>().ok().filter(|v| v.is_finite())
}

fn has_header(text: &str, header: &str) -> bool {
    text.trim_start_matches('\u{feff}').lines().next().map(str::trim) == Some(header)
}

pub fn parse_position(text: &str) -> Option<Position> {
    if !has_header(text, POSITION_HEADER) {
        return None;
    }
    let v = |key| field(text, key).and_then(number);
    let p = Position { mid: [v("x_mid")?, v("y_mid")?, v("z_mid")?], z_start: v("z_start")?, z_end: v("z_end")?, zoom: v("zoom")? };
    (p.zoom > 0.0).then_some(p)
}

pub fn parse_rotation(text: &str) -> Option<Mat3> {
    if !has_header(text, ROTATION_HEADER) {
        return None;
    }
    let row = |key| -> Option<[f64; 3]> {
        let mut it = field(text, key)?.split_whitespace().map(number);
        let r = [it.next()??, it.next()??, it.next()??];
        it.next().is_none().then_some(r)
    };
    let m = [row("row1")?, row("row2")?, row("row3")?];
    // a flat matrix would leave no picture to look at
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    (det.abs() > 1e-9).then_some(m)
}

/// The file's text; None when it is too big to be one of these files or is
/// no text.
pub fn read_text(path: &std::path::Path) -> std::io::Result<Option<String>> {
    use std::io::Read;
    let mut data = Vec::new();
    std::fs::File::open(path)?.take(MAX_BYTES + 1).read_to_end(&mut data)?;
    if data.len() as u64 > MAX_BYTES {
        return Ok(None);
    }
    Ok(String::from_utf8(data).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: Position = Position { mid: [0.1 + 0.2, -1.234567890123456e-300, 5e300], z_start: -6.000000000000001, z_end: 120.0, zoom: 1.7976931348623157e308 };
    const M: Mat3 = [[2.735478468258491e-30, 0.1 + 0.7, 1.0 / 3.0], [-1.0, 0.0, 5e-324], [0.0, -0.0, 1.0]];

    fn bits(m: &Mat3) -> Vec<u64> {
        m.iter().flatten().map(|v| v.to_bits()).collect()
    }

    #[test]
    fn position_comes_back_to_the_bit() {
        let r = parse_position(&position_text(&P)).unwrap();
        for (a, b) in [P.mid[0], P.mid[1], P.mid[2], P.z_start, P.z_end, P.zoom].iter().zip([r.mid[0], r.mid[1], r.mid[2], r.z_start, r.z_end, r.zoom]) {
            assert_eq!(a.to_bits(), b.to_bits());
        }
    }

    #[test]
    fn rotation_comes_back_to_the_bit() {
        // (the view straight along y, where the panel's angles read "?")
        let m = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        assert_eq!(bits(&parse_rotation(&rotation_text(&m)).unwrap()), bits(&m));
        let mut m = M;
        m[1][2] = 0.5; // (a matrix that is not flat)
        assert_eq!(bits(&parse_rotation(&rotation_text(&m)).unwrap()), bits(&m));
    }

    #[test]
    fn windows_line_ends_and_a_bom_are_read() {
        let t = format!("\u{feff}{}", position_text(&P).replace('\n', "\r\n"));
        assert_eq!(parse_position(&t).unwrap().z_end, 120.0);
    }

    #[test]
    fn a_file_of_the_other_kind_is_refused() {
        assert!(parse_position(&rotation_text(&[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])).is_none());
        assert!(parse_rotation(&position_text(&P)).is_none());
        assert!(parse_position("Mandelbulb3Dv18{ABC}").is_none());
        assert!(parse_position("").is_none());
    }

    #[test]
    fn broken_values_are_refused() {
        let good = position_text(&P);
        assert!(parse_position(&good.replace("z_end = 120.0\n", "")).is_none(), "missing line");
        assert!(parse_position(&good.replace("z_end = 120.0", "z_end = NaN")).is_none());
        assert!(parse_position(&good.replace("z_end = 120.0", "z_end = inf")).is_none());
        assert!(parse_position(&good.replace("z_end = 120.0", "z_end = 1,5")).is_none());
        let zoom0 = Position { zoom: 0.0, ..P };
        assert!(parse_position(&position_text(&zoom0)).is_none());
        let i = "row1 = 1.0 0.0 0.0\nrow2 = 0.0 1.0 0.0\nrow3 = 0.0 0.0 1.0\n";
        assert!(parse_rotation(&format!("{ROTATION_HEADER}\n{i}")).is_some());
        assert!(parse_rotation(&format!("{ROTATION_HEADER}\n{}", i.replace("row2 = 0.0 1.0 0.0", "row2 = 0.0 1.0"))).is_none(), "short row");
        assert!(parse_rotation(&format!("{ROTATION_HEADER}\n{}", i.replace("row2 = 0.0 1.0 0.0", "row2 = 0.0 1.0 0.0 7.0"))).is_none(), "long row");
        assert!(parse_rotation(&format!("{ROTATION_HEADER}\n{}", i.replace("row2 = 0.0 1.0 0.0", "row2 = 1.0 0.0 0.0"))).is_none(), "flat matrix");
    }

    #[test]
    fn a_big_or_binary_file_is_not_read_as_text() {
        let d = std::env::temp_dir().join(format!("mb3d-camera-file-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let big = d.join("big.m3xyz");
        std::fs::write(&big, vec![b'a'; MAX_BYTES as usize + 1]).unwrap();
        assert_eq!(read_text(&big).unwrap(), None);
        let bin = d.join("bin.m3xyz");
        std::fs::write(&bin, [0xff, 0xfe, 0x00]).unwrap();
        assert_eq!(read_text(&bin).unwrap(), None);
        let ok = d.join("ok.m3xyz");
        std::fs::write(&ok, position_text(&P)).unwrap();
        assert_eq!(parse_position(&read_text(&ok).unwrap().unwrap()).unwrap().z_end, 120.0);
        let _ = std::fs::remove_dir_all(&d);
    }
}
