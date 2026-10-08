// decodes images and writes raw RGB8 dumps: imgdump in... (writes in.rgb)
fn main() {
    for f in std::env::args().skip(1) {
        match mb3d::image::load(std::path::Path::new(&f)) {
            Ok(img) => {
                let mut out = format!("{} {}\n", img.width, img.height).into_bytes();
                for p in &img.data {
                    for c in p {
                        out.push(if img.deep { (c >> 8) as u8 } else { *c as u8 });
                    }
                }
                std::fs::write(format!("/tmp/imgdump/{}.rgb", std::path::Path::new(&f).file_name().unwrap().to_string_lossy()), out).unwrap();
            }
            Err(e) => println!("ERR {f}: {e}"),
        }
    }
}
