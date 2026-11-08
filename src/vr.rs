//! 360 degree pictures and videos for VR headsets and 360 viewers (not in
//! MB3D).
//!
//! A scene with the camera optic "spherical panorama" is an equirectangular
//! picture (2:1). Viewers only show it as a sphere when they are told so:
//!
//! * Photos: the Photo Sphere XMP of a JPEG (`GPano`, Google's tags that
//!   Facebook, Google Photos and most 360 viewers read): [`jpeg_with_pano`],
//!   and with [`camera_tag`] on, the EXIF camera of a 360 camera (Make
//!   `RICOH`, Model `RICOH THETA S`), the tag Facebook's own upload goes by
//!   when it ignores the XMP (the trick of the Mandelbulb Maniacs group,
//!   who set it by hand in the file's properties). A PNG gets both as an
//!   `eXIf` and an XMP `iTXt` chunk: [`png_with_pano`].
//! * Videos: the Spherical Video V2 boxes of an MP4 (`st3d` for the stereo
//!   layout, `sv3d` with an equirectangular `proj`; YouTube, Meta Quest's
//!   players, VLC): [`inject_spherical_mp4`]. Players that ignore them (and
//!   AVI files, which have no place for them) go by the file name:
//!   `_360` and, for a stereo video, `_360_TB` ([`vr_suffix`]).
//! * Stereo: the eyes one above the other, the left on top (top-bottom,
//!   as YouTube and the Quest expect): [`compose_top_bottom`].

use crate::anim::{Animation, OutputFormat};
use std::path::{Path, PathBuf};

/// The stereo layout of a 360 video (Spherical Video V2 `stereo_mode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stereo {
    Mono = 0,
    TopBottom = 1,
    LeftRight = 2,
}

impl Stereo {
    pub fn parse(s: &str) -> Result<Stereo, String> {
        match s.to_ascii_lowercase().as_str() {
            "mono" | "none" | "off" => Ok(Stereo::Mono),
            "tb" | "top-bottom" | "topbottom" | "over-under" => Ok(Stereo::TopBottom),
            "lr" | "sbs" | "left-right" | "side-by-side" => Ok(Stereo::LeftRight),
            o => Err(format!("stereo layout '{o}' (mono, tb or lr)")),
        }
    }
}

/// The file name tag 360 players look for: `_360`, `_360_TB`, `_360_LR`.
pub fn vr_suffix(st: Stereo) -> &'static str {
    match st {
        Stereo::Mono => "_360",
        Stereo::TopBottom => "_360_TB",
        Stereo::LeftRight => "_360_LR",
    }
}

/// `stem` with the [`vr_suffix`] unless it ends with one already.
pub fn vr_name(stem: &str, st: Stereo) -> String {
    let l = stem.to_ascii_lowercase();
    if ["_360", "_360_tb", "_360_lr", "_tb_360", "_lr_360"].iter().any(|s| l.ends_with(s)) {
        stem.to_string()
    } else {
        format!("{stem}{}", vr_suffix(st))
    }
}

/// Is a picture of `w` × `h` of this scene an equirectangular 360 one?
pub fn is_pano(sc: &crate::scene::Scene, w: usize, h: usize) -> bool {
    sc.optic == crate::scene::CameraOptic::Panorama && w == 2 * h
}

// ---------------------------------------------------------------------------
// photos

/// The Photo Sphere XMP packet of a whole `w` × `h` equirectangular picture.
pub fn pano_xmp(w: usize, h: usize) -> String {
    format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"mb3d-rs\">\
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description rdf:about=\"\" xmlns:GPano=\"http://ns.google.com/photos/1.0/panorama/\" \
GPano:ProjectionType=\"equirectangular\" GPano:UsePanoramaViewer=\"True\" \
GPano:CroppedAreaImageWidthPixels=\"{w}\" GPano:CroppedAreaImageHeightPixels=\"{h}\" \
GPano:FullPanoWidthPixels=\"{w}\" GPano:FullPanoHeightPixels=\"{h}\" \
GPano:CroppedAreaLeftPixels=\"0\" GPano:CroppedAreaTopPixels=\"0\" \
GPano:PoseHeadingDegrees=\"0\" GPano:InitialViewHeadingDegrees=\"0\"/>\
</rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>"
    )
}

/// Whether 360 pictures also get the EXIF camera of a 360 camera
/// ([`pano_exif`]); on unless the user switched it off (the Share window,
/// `Pano360CameraTag` in the ini file).
static CAMERA_TAG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Switches the 360 camera tag of [`pano_exif`] on or off.
pub fn set_camera_tag(on: bool) {
    CAMERA_TAG.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Is the 360 camera tag on?
pub fn camera_tag() -> bool {
    CAMERA_TAG.load(std::sync::atomic::Ordering::Relaxed)
}

/// The camera Facebook takes for a 360 camera (Make, Model).
pub const PANO_CAMERA: (&str, &str) = ("RICOH", "RICOH THETA S");

/// The EXIF block (a little-endian TIFF structure, without the `Exif\0\0`
/// of a JPEG) of a 360 picture: IFD0 with Make and Model of
/// [`PANO_CAMERA`] and Software (this program and its version).
pub fn pano_exif() -> Vec<u8> {
    let software = format!("mb3d-rs {}", env!("CARGO_PKG_VERSION"));
    // (tag, ASCII value with its terminating zero), sorted by tag
    let fields: [(u16, String); 3] = [(0x010F, PANO_CAMERA.0.to_string()), (0x0110, PANO_CAMERA.1.to_string()), (0x0131, software)];
    let n = fields.len();
    let ifd = 8;
    let mut data_at = ifd + 2 + n * 12 + 4;
    let mut t: Vec<u8> = vec![b'I', b'I', 42, 0];
    t.extend_from_slice(&(ifd as u32).to_le_bytes());
    t.extend_from_slice(&(n as u16).to_le_bytes());
    let mut values: Vec<u8> = Vec::new();
    for (tag, v) in &fields {
        let mut bytes = v.as_bytes().to_vec();
        bytes.push(0);
        t.extend_from_slice(&tag.to_le_bytes());
        t.extend_from_slice(&2u16.to_le_bytes()); // ASCII
        t.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        if bytes.len() <= 4 {
            let mut inline = bytes.clone();
            inline.resize(4, 0);
            t.extend_from_slice(&inline);
        } else {
            t.extend_from_slice(&(data_at as u32).to_le_bytes());
            data_at += bytes.len() + bytes.len() % 2;
            values.extend_from_slice(&bytes);
            if bytes.len() % 2 == 1 {
                values.push(0);
            }
        }
    }
    t.extend_from_slice(&0u32.to_le_bytes()); // no next IFD
    t.extend_from_slice(&values);
    t
}

/// A JPEG file with the Photo Sphere XMP of a `w` × `h` picture in an APP1
/// segment after the JFIF one (an XMP segment there already is replaced),
/// and with [`camera_tag`] on the EXIF segment of [`pano_exif`] before it
/// (an EXIF segment there already is replaced as well).
pub fn jpeg_with_pano(jpeg: &[u8], w: usize, h: usize) -> Result<Vec<u8>, String> {
    const NS: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
    const EXIF: &[u8] = b"Exif\0\0";
    if !jpeg.starts_with(&[0xFF, 0xD8]) {
        return Err("not a JPEG file".into());
    }
    // the segments before the image data: kept, an XMP one left out
    let mut pos = 2;
    let mut head: Vec<u8> = Vec::new();
    let mut insert_at = None;
    while pos + 4 <= jpeg.len() && jpeg[pos] == 0xFF {
        let m = jpeg[pos + 1];
        if !(0xE0..=0xEF).contains(&m) && m != 0xFE {
            break;
        }
        let len = u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]) as usize;
        let seg = jpeg.get(pos..pos + 2 + len).ok_or("a JPEG segment is cut off")?;
        let xmp = m == 0xE1 && seg.get(4..4 + NS.len()) == Some(NS);
        let exif = m == 0xE1 && seg.get(4..4 + EXIF.len()) == Some(EXIF);
        if !xmp && !exif {
            head.extend_from_slice(seg);
        }
        if m == 0xE0 && insert_at.is_none() {
            insert_at = Some(head.len());
        }
        pos += 2 + len;
    }
    let packet = pano_xmp(w, h);
    let len = 2 + NS.len() + packet.len();
    if len > 0xFFFF {
        return Err("XMP too long".into());
    }
    let mut seg = Vec::new();
    if camera_tag() {
        let e = pano_exif();
        seg.extend_from_slice(&[0xFF, 0xE1]);
        seg.extend_from_slice(&((2 + EXIF.len() + e.len()) as u16).to_be_bytes());
        seg.extend_from_slice(EXIF);
        seg.extend_from_slice(&e);
    }
    seg.extend_from_slice(&[0xFF, 0xE1]);
    seg.extend_from_slice(&(len as u16).to_be_bytes());
    seg.extend_from_slice(NS);
    seg.extend_from_slice(packet.as_bytes());
    let at = insert_at.unwrap_or(0);
    let mut out = Vec::with_capacity(jpeg.len() + seg.len());
    out.extend_from_slice(&[0xFF, 0xD8]);
    out.extend_from_slice(&head[..at]);
    out.extend_from_slice(&seg);
    out.extend_from_slice(&head[at..]);
    out.extend_from_slice(&jpeg[pos..]);
    Ok(out)
}

/// Encodes a picture as JPEG, with the Photo Sphere XMP when it is a 360
/// one of `sc` ([`is_pano`]).
pub fn encode_jpeg_for(sc: &crate::scene::Scene, w: usize, h: usize, rgb: &[u8], quality: u8) -> Vec<u8> {
    let j = crate::jpeg::encode(w, h, rgb, quality);
    if is_pano(sc, w, h) {
        jpeg_with_pano(&j, w, h).unwrap_or(j)
    } else {
        j
    }
}

/// A PNG file (from [`crate::png`]'s encoders) with the 360 tags of a
/// `w` × `h` picture: the Photo Sphere XMP as an `iTXt` chunk
/// (`XML:com.adobe.xmp`), and with [`camera_tag`] on the EXIF block of
/// [`pano_exif`] as an `eXIf` chunk, both before the image data.
pub fn png_with_pano(png: Vec<u8>, w: usize, h: usize) -> Vec<u8> {
    let mut xmp = b"XML:com.adobe.xmp\0\0\0\0\0".to_vec();
    xmp.extend_from_slice(pano_xmp(w, h).as_bytes());
    let mut png = crate::png::with_chunk_before_idat(png, b"iTXt", &xmp);
    if camera_tag() {
        png = crate::png::with_chunk_before_idat(png, b"eXIf", &pano_exif());
    }
    png
}

/// Encodes a picture as PNG, with the 360 tags when it is a 360 one of
/// `sc` ([`is_pano`]).
pub fn encode_png_for(sc: &crate::scene::Scene, w: usize, h: usize, rgb: &[u8]) -> Vec<u8> {
    let p = crate::png::encode_rgb(w, h, rgb);
    if is_pano(sc, w, h) {
        png_with_pano(p, w, h)
    } else {
        p
    }
}

/// A PNG or JPEG file (by the extension of `path`; anything but `.jpg` and
/// `.jpeg` is a PNG) of a picture of `sc`, with the 360 tags when it is a
/// 360 one.
pub fn encode_picture_for(sc: &crate::scene::Scene, path: &Path, w: usize, h: usize, rgb: &[u8], quality: u8) -> Vec<u8> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if ext == "jpg" || ext == "jpeg" {
        encode_jpeg_for(sc, w, h, rgb, quality)
    } else {
        encode_png_for(sc, w, h, rgb)
    }
}

// ---------------------------------------------------------------------------
// stereo frames

/// The left eye's picture above the right one's.
pub fn compose_top_bottom(left: &crate::image::Image, right: &crate::image::Image) -> Result<(Vec<u8>, usize, usize), String> {
    if left.width != right.width || left.height != right.height {
        return Err(format!("the eyes' pictures differ in size ({}x{} and {}x{})", left.width, left.height, right.width, right.height));
    }
    let to8 = |im: &crate::image::Image| -> Vec<u8> {
        im.data.iter().flat_map(|p| p.map(|v| if im.deep { (v >> 8) as u8 } else { v as u8 })).collect()
    };
    let mut rgb = to8(left);
    rgb.extend(to8(right));
    Ok((rgb, left.width, left.height * 2))
}

/// The frame file of a stereo 360 animation's top-bottom picture
/// (`<name>VR<index>.<ext>`).
pub fn vr_frame_file(a: &Animation, frame: usize) -> PathBuf {
    a.frame_file_eye(frame, "VR")
}

/// The pictures of `frames` that a video is made of: the frames, or of a
/// stereo 360 animation the two eyes put together top-bottom (written as
/// [`vr_frame_file`], those there already newer than the eyes kept).
/// Frames whose pictures are missing are left out.
pub fn video_frames(a: &Animation, frames: &[usize]) -> Result<Vec<PathBuf>, String> {
    let eyes = a.stereo_eyes();
    if eyes.len() == 1 {
        return Ok(frames.iter().map(|&f| a.frame_file(f)).filter(|p| p.is_file()).collect());
    }
    if !a.vr360 {
        return Err("a video of a stereo animation needs its 360 degree VR option (the eyes are put one above the other)".into());
    }
    let mut out = Vec::new();
    for &f in frames {
        let (l, r) = (a.frame_file_eye(f, "Left"), a.frame_file_eye(f, "Right"));
        if !l.is_file() || !r.is_file() {
            continue;
        }
        let vr = vr_frame_file(a, f);
        let mtime = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
        let fresh = match (mtime(&vr), mtime(&l), mtime(&r)) {
            (Some(v), Some(a), Some(b)) => v >= a && v >= b,
            _ => false,
        };
        if !fresh {
            let (rgb, w, h) = compose_top_bottom(&crate::image::load(&l)?, &crate::image::load(&r)?)?;
            let data = match a.format {
                OutputFormat::Bmp => crate::frames::encode_bmp(w, h, &rgb),
                OutputFormat::Jpg => crate::jpeg::encode(w, h, &rgb, 95),
                _ => crate::png::encode_rgb(w, h, &rgb),
            };
            std::fs::write(&vr, data).map_err(|e| format!("{}: {e}", vr.display()))?;
        }
        out.push(vr);
    }
    Ok(out)
}

/// The stereo layout of a 360 animation's video.
pub fn anim_stereo(a: &Animation) -> Stereo {
    if a.stereo_eyes().len() == 2 {
        Stereo::TopBottom
    } else {
        Stereo::Mono
    }
}

// ---------------------------------------------------------------------------
// MP4: Spherical Video V2

fn be32(b: &[u8], i: usize) -> u32 {
    u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

/// The boxes of `b`: (type, start, header length, end).
fn boxes(b: &[u8]) -> Result<Vec<([u8; 4], usize, usize, usize)>, String> {
    let mut v = Vec::new();
    let mut p = 0;
    while p + 8 <= b.len() {
        let size = be32(b, p) as u64;
        let ty = [b[p + 4], b[p + 5], b[p + 6], b[p + 7]];
        let (hl, size) = match size {
            0 => (8, (b.len() - p) as u64),
            1 => {
                if p + 16 > b.len() {
                    return Err("a box is cut off".into());
                }
                (16, u64::from_be_bytes(b[p + 8..p + 16].try_into().unwrap()))
            }
            s => (8, s),
        };
        if size < hl as u64 || p as u64 + size > b.len() as u64 {
            return Err(format!("box '{}' does not fit in its file", String::from_utf8_lossy(&ty)));
        }
        v.push((ty, p, hl, p + size as usize));
        p += size as usize;
    }
    Ok(v)
}

fn make_box(ty: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut o = Vec::with_capacity(body.len() + 8);
    o.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
    o.extend_from_slice(ty);
    o.extend_from_slice(body);
    o
}

fn full_box(ty: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut b = vec![0u8; 4];
    b.extend_from_slice(body);
    make_box(ty, &b)
}

/// The `st3d` and `sv3d` boxes of an equirectangular video.
fn spherical_boxes(st: Stereo) -> Vec<u8> {
    let mut v = full_box(b"st3d", &[st as u8]);
    let svhd = full_box(b"svhd", b"mb3d-rs\0");
    let prhd = full_box(b"prhd", &[0u8; 12]);
    let equi = full_box(b"equi", &[0u8; 16]);
    let proj = make_box(b"proj", &[prhd, equi].concat());
    v.extend(make_box(b"sv3d", &[svhd, proj].concat()));
    v
}

/// The uuid of the Spherical Video V1 metadata box.
const SPHERICAL_V1_UUID: [u8; 16] = [0xff, 0xcc, 0x82, 0x63, 0xf8, 0x55, 0x4a, 0x93, 0x88, 0x14, 0x58, 0x7a, 0x02, 0x52, 0x1f, 0xdd];

/// The Spherical Video V1 box of an equirectangular video in the layout
/// `st` (Google's spherical video RFC, the XML of its injector).
fn spherical_v1_box(st: Stereo) -> Vec<u8> {
    let stereo = match st {
        Stereo::Mono => String::new(),
        Stereo::TopBottom => "<GSpherical:StereoMode>top-bottom</GSpherical:StereoMode>".to_string(),
        Stereo::LeftRight => "<GSpherical:StereoMode>left-right</GSpherical:StereoMode>".to_string(),
    };
    let xml = format!(
        "<?xml version=\"1.0\"?><rdf:SphericalVideo\nxmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"\nxmlns:GSpherical=\"http://ns.google.com/videos/1.0/spherical/\">\
<GSpherical:Spherical>true</GSpherical:Spherical>\
<GSpherical:Stitched>true</GSpherical:Stitched>\
<GSpherical:StitchingSoftware>Spherical Metadata Tool</GSpherical:StitchingSoftware>\
<GSpherical:ProjectionType>equirectangular</GSpherical:ProjectionType>{stereo}</rdf:SphericalVideo>"
    );
    let mut body = SPHERICAL_V1_UUID.to_vec();
    body.extend_from_slice(xml.as_bytes());
    make_box(b"uuid", &body)
}

const VIDEO_ENTRIES: [&[u8; 4]; 7] = [b"avc1", b"avc3", b"hvc1", b"hev1", b"mp4v", b"vp09", b"av01"];
/// The sample entry's fields before its boxes (VisualSampleEntry).
const VISUAL_ENTRY_FIELDS: usize = 78;

/// The boxes of `b` rewritten: the containers on the way to the video
/// sample entries entered, `st3d`/`sv3d` (old ones dropped) appended to
/// those, the chunk offsets moved by `delta`. Returns how many video
/// sample entries it found.
fn rewrite(b: &[u8], st: Stereo, delta: i64, found: &mut usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(b.len() + 128);
    for (ty, s, hl, e) in boxes(b)? {
        let body = &b[s + hl..e];
        match &ty {
            b"trak" => {
                // a video track also gets the Spherical Video V1 metadata
                // (an XML uuid box in the track: what Google's Spatial Media
                // Metadata Injector writes; players such as Facebook's may only look
                // for this one)
                let before = *found;
                let inner = rewrite(body, st, delta, found)?;
                if *found > before {
                    let mut t = Vec::with_capacity(inner.len() + 600);
                    for (cty, cs, chl, ce) in boxes(&inner)? {
                        let v1 = &cty == b"uuid" && inner.get(cs + chl..cs + chl + 16) == Some(&SPHERICAL_V1_UUID[..]);
                        if !v1 {
                            t.extend_from_slice(&inner[cs..ce]);
                        }
                    }
                    t.extend(spherical_v1_box(st));
                    out.extend(make_box(&ty, &t));
                } else {
                    out.extend(make_box(&ty, &inner));
                }
            }
            b"moov" | b"mdia" | b"minf" | b"stbl" => {
                out.extend(make_box(&ty, &rewrite(body, st, delta, found)?));
            }
            b"stsd" => {
                if body.len() < 8 {
                    return Err("stsd too short".into());
                }
                let mut nb = body[..8].to_vec();
                for (ety, es, ehl, ee) in boxes(&body[8..])? {
                    let entry = &body[8 + es + ehl..8 + ee];
                    if VIDEO_ENTRIES.contains(&&ety) && entry.len() >= VISUAL_ENTRY_FIELDS {
                        *found += 1;
                        let mut eb = entry[..VISUAL_ENTRY_FIELDS].to_vec();
                        for (cty, cs, _, ce) in boxes(&entry[VISUAL_ENTRY_FIELDS..])? {
                            if &cty != b"st3d" && &cty != b"sv3d" {
                                eb.extend_from_slice(&entry[VISUAL_ENTRY_FIELDS + cs..VISUAL_ENTRY_FIELDS + ce]);
                            }
                        }
                        eb.extend(spherical_boxes(st));
                        nb.extend(make_box(&ety, &eb));
                    } else {
                        nb.extend_from_slice(&body[8 + es..8 + ee]);
                    }
                }
                out.extend(make_box(&ty, &nb));
            }
            b"stco" | b"co64" if delta != 0 => {
                let wide = &ty == b"co64";
                if body.len() < 8 {
                    return Err("chunk offsets too short".into());
                }
                let n = be32(body, 4) as usize;
                let w = if wide { 8 } else { 4 };
                if body.len() < 8 + n * w {
                    return Err("chunk offsets cut off".into());
                }
                let mut nb = body[..8].to_vec();
                for i in 0..n {
                    let p = 8 + i * w;
                    if wide {
                        let v = u64::from_be_bytes(body[p..p + 8].try_into().unwrap()) as i64 + delta;
                        nb.extend_from_slice(&(v as u64).to_be_bytes());
                    } else {
                        let v = be32(body, p) as i64 + delta;
                        if !(0..=u32::MAX as i64).contains(&v) {
                            return Err("the file is too large for its 32 bit chunk offsets".into());
                        }
                        nb.extend_from_slice(&(v as u32).to_be_bytes());
                    }
                }
                out.extend(make_box(&ty, &nb));
            }
            _ => out.extend_from_slice(&b[s..e]),
        }
    }
    Ok(out)
}

/// An MP4 with the Spherical Video V2 metadata of an equirectangular video
/// in the layout `st` (every video track; metadata there already
/// replaced). The media data stays as it is; where the movie header comes
/// before it, the chunk offsets move with the header's growth.
pub fn inject_spherical_mp4(mp4: &[u8], st: Stereo) -> Result<Vec<u8>, String> {
    let top = boxes(mp4)?;
    let moov = top.iter().find(|b| &b.0 == b"moov").ok_or("no movie header ('moov'): not an MP4 file")?;
    let mdat_after = top.iter().any(|b| &b.0 == b"mdat" && b.1 > moov.1);
    let mut found = 0;
    let first = rewrite(&mp4[moov.1..moov.3], st, 0, &mut found)?;
    if found == 0 {
        return Err("no video track (H.264, H.265, MPEG-4, VP9 or AV1) found".into());
    }
    let delta = first.len() as i64 - (moov.3 - moov.1) as i64;
    let new_moov = if mdat_after && delta != 0 {
        let mut f = 0;
        rewrite(&mp4[moov.1..moov.3], st, delta, &mut f)?
    } else {
        first
    };
    let mut out = Vec::with_capacity(mp4.len() + 256);
    out.extend_from_slice(&mp4[..moov.1]);
    out.extend(new_moov);
    out.extend_from_slice(&mp4[moov.3..]);
    Ok(out)
}

/// A file made into a 360 one in place or to `out`: an MP4 gets the
/// Spherical Video V2 metadata, a JPEG the Photo Sphere XMP (a mono
/// picture: Facebook and photo viewers take no stereo 360 photos).
pub fn mark_file(path: &Path, out: &Path, st: Stereo) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (res, what) = if data.starts_with(&[0xFF, 0xD8]) {
        let im = crate::image::decode(&data)?;
        if im.width != 2 * im.height {
            return Err(format!("{}: {}x{} is no 360 degree panorama (2:1)", path.display(), im.width, im.height));
        }
        (jpeg_with_pano(&data, im.width, im.height)?, "360 photo (Photo Sphere XMP)".to_string())
    } else {
        (inject_spherical_mp4(&data, st)?, format!("360 video, {} (Spherical Video V1 and V2)", match st {
            Stereo::Mono => "mono",
            Stereo::TopBottom => "stereo top-bottom",
            Stereo::LeftRight => "stereo left-right",
        }))
    };
    std::fs::write(out, res).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(what)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mp4(moov_first: bool) -> Vec<u8> {
        let ftyp = make_box(b"ftyp", b"isom\0\0\x02\0isomavc1");
        let mut entry = vec![0u8; VISUAL_ENTRY_FIELDS];
        entry.extend(make_box(b"avcC", &[1, 2, 3]));
        let mut stsd = vec![0, 0, 0, 0, 0, 0, 0, 1];
        stsd.extend(make_box(b"avc1", &entry));
        let mdat = make_box(b"mdat", &[7u8; 40]);
        let build = |off: u32| {
            let mut stco = vec![0, 0, 0, 0, 0, 0, 0, 1];
            stco.extend_from_slice(&off.to_be_bytes());
            let stbl = make_box(b"stbl", &[make_box(b"stsd", &stsd), make_box(b"stco", &stco)].concat());
            let minf = make_box(b"minf", &stbl);
            let mdia = make_box(b"mdia", &minf);
            let trak = make_box(b"trak", &mdia);
            make_box(b"moov", &trak)
        };
        if moov_first {
            let l = build(0).len();
            let off = (ftyp.len() + l + 8) as u32;
            [ftyp, build(off), mdat].concat()
        } else {
            let off = (ftyp.len() + 8) as u32;
            [ftyp, mdat, build(off)].concat()
        }
    }

    /// The offset of the first chunk and the data it points at.
    fn first_chunk(f: &[u8]) -> (usize, u8) {
        let i = f.windows(4).position(|w| w == b"stco").unwrap();
        let off = be32(f, i + 12) as usize;
        (off, f[off])
    }

    #[test]
    fn the_spherical_boxes_go_into_the_video_entry() {
        for moov_first in [false, true] {
            let f = mp4(moov_first);
            assert_eq!(first_chunk(&f).1, 7);
            let g = inject_spherical_mp4(&f, Stereo::TopBottom).unwrap();
            assert!(g.windows(4).any(|w| w == b"sv3d") && g.windows(4).any(|w| w == b"equi"));
            // and the V1 XML in the track, once
            assert_eq!(g.windows(16).filter(|w| *w == SPHERICAL_V1_UUID).count(), 1);
            assert!(String::from_utf8_lossy(&g).contains("<GSpherical:StereoMode>top-bottom</GSpherical:StereoMode>"));
            let i = g.windows(4).position(|w| w == b"st3d").unwrap();
            assert_eq!(g[i + 8], 1, "top-bottom");
            // every size still adds up, and the chunk still finds its data
            let mut found = 0;
            rewrite(&g[boxes(&g).unwrap().iter().find(|b| &b.0 == b"moov").unwrap().1..], Stereo::Mono, 0, &mut found).unwrap();
            assert_eq!(found, 1);
            assert_eq!(first_chunk(&g).1, 7, "moov first: {moov_first}");
            // a second time replaces, does not add
            let h = inject_spherical_mp4(&g, Stereo::TopBottom).unwrap();
            assert_eq!(h.len(), g.len());
            assert_eq!(h.windows(4).filter(|w| w == b"sv3d").count(), 1);
            assert_eq!(h.windows(16).filter(|w| *w == SPHERICAL_V1_UUID).count(), 1);
            assert_eq!(first_chunk(&h).1, 7);
        }
    }

    #[test]
    fn a_jpeg_gets_the_photo_sphere_xmp() {
        let rgb = vec![128u8; 64 * 32 * 3];
        let j = crate::jpeg::encode(64, 32, &rgb, 90);
        let p = jpeg_with_pano(&j, 64, 32).unwrap();
        let s = String::from_utf8_lossy(&p);
        assert!(s.contains("GPano:ProjectionType=\"equirectangular\"") && s.contains("FullPanoWidthPixels=\"64\""));
        // still a picture of the same size, and no second packet when done again
        let im = crate::image::decode(&p).unwrap();
        assert_eq!((im.width, im.height), (64, 32));
        let q = jpeg_with_pano(&p, 64, 32).unwrap();
        assert_eq!(q.len(), p.len());
        // after the JFIF segment
        assert_eq!(&p[2..4], &[0xFF, 0xE0]);
    }

    #[test]
    fn a_360_jpeg_gets_the_camera_of_a_360_camera() {
        let rgb = vec![90u8; 64 * 32 * 3];
        let j = crate::jpeg::encode(64, 32, &rgb, 90);
        let p = jpeg_with_pano(&j, 64, 32).unwrap();
        // the EXIF segment right after the JFIF one, the XMP after it
        let at = |b: &[u8], i: usize| (b[i], b[i + 1], u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize);
        let (_, m0, l0) = at(&p, 2);
        assert_eq!(m0, 0xE0);
        let (_, m1, l1) = at(&p, 4 + l0);
        assert_eq!(m1, 0xE1);
        assert_eq!(&p[8 + l0..14 + l0], b"Exif\0\0");
        let (_, m2, _) = at(&p, 6 + l0 + l1);
        assert_eq!(m2, 0xE1);
        assert_eq!(&p[10 + l0 + l1..10 + l0 + l1 + 28], b"http://ns.adobe.com/xap/1.0/");
        // done again: one of each
        let q = jpeg_with_pano(&p, 64, 32).unwrap();
        assert_eq!(q.len(), p.len());
        assert_eq!(q.windows(6).filter(|w| *w == b"Exif\0\0").count(), 1);
        let im = crate::image::decode(&q).unwrap();
        assert_eq!((im.width, im.height), (64, 32));
    }

    #[test]
    fn the_exif_block_reads_as_tiff() {
        let t = pano_exif();
        assert_eq!(&t[..4], b"II*\0");
        let u16at = |i: usize| u16::from_le_bytes([t[i], t[i + 1]]);
        let u32at = |i: usize| u32::from_le_bytes(t[i..i + 4].try_into().unwrap());
        let ifd = u32at(4) as usize;
        let n = u16at(ifd) as usize;
        assert_eq!(n, 3);
        let mut tags = Vec::new();
        for k in 0..n {
            let e = ifd + 2 + k * 12;
            let (tag, ty, count, off) = (u16at(e), u16at(e + 2), u32at(e + 4) as usize, u32at(e + 8) as usize);
            assert_eq!(ty, 2);
            let v = &t[off..off + count];
            assert_eq!(v[count - 1], 0);
            tags.push((tag, String::from_utf8(v[..count - 1].to_vec()).unwrap()));
        }
        assert_eq!(tags[0], (0x010F, "RICOH".to_string()));
        assert_eq!(tags[1], (0x0110, "RICOH THETA S".to_string()));
        assert!(tags[2].1.starts_with("mb3d-rs "));
        // tags in ascending order, no next IFD
        assert!(tags.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(u32at(ifd + 2 + n * 12), 0);
    }

    #[test]
    fn a_360_png_gets_xmp_and_exif_before_the_image_data() {
        let p = png_with_pano(crate::png::encode_rgb(8, 4, &[7u8; 8 * 4 * 3]), 8, 4);
        let find = |k: &[u8]| p.windows(4).position(|w| w == k).unwrap();
        let (itxt, exif, idat) = (find(b"iTXt"), find(b"eXIf"), find(b"IDAT"));
        assert!(itxt < idat && exif < idat);
        assert!(String::from_utf8_lossy(&p).contains("GPano:ProjectionType=\"equirectangular\""));
        let im = crate::image::decode(&p).unwrap();
        assert_eq!((im.width, im.height), (8, 4));
    }

    #[test]
    fn names_get_the_players_tag_once() {
        assert_eq!(vr_name("flight", Stereo::TopBottom), "flight_360_TB");
        assert_eq!(vr_name("flight_360_TB", Stereo::TopBottom), "flight_360_TB");
        assert_eq!(vr_name("orbit", Stereo::Mono), "orbit_360");
    }
}
