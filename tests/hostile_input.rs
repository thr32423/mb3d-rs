//! Damaged or hostile files end in an error, never in a crash, an endless
//! read or a file opened elsewhere: the cases found by the review of
//! v0.3.6 (each failed on that version: a panic, an allocation abort or a
//! read for hours).

use std::path::PathBuf;

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mb3d_hostile_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn pictures_of_no_size_are_errors() {
    // (a division by the width when the picture became a map)
    assert!(mb3d::image::decode(b"P5 0 1 255\n").is_err());
    assert!(mb3d::image::decode(b"P5 1 0 255\n").is_err());
    let mut bmp = vec![0u8; 54];
    bmp[0..2].copy_from_slice(b"BM");
    bmp[10..14].copy_from_slice(&54u32.to_le_bytes());
    bmp[14..18].copy_from_slice(&40u32.to_le_bytes());
    bmp[18..22].copy_from_slice(&0i32.to_le_bytes()); // width 0
    bmp[22..26].copy_from_slice(&4i32.to_le_bytes());
    bmp[26..28].copy_from_slice(&1u16.to_le_bytes());
    bmp[28..30].copy_from_slice(&24u16.to_le_bytes());
    assert!(mb3d::image::decode(&bmp).is_err());
    // a real one still reads
    let ok = mb3d::image::decode(b"P5 2 1 255\n\x00\xff").unwrap();
    assert_eq!((ok.width, ok.height), (2, 1));
}

#[test]
fn a_jpeg_header_cannot_ask_for_gigabytes() {
    // 65535 x 65535, three components: about 50 GB of blocks
    let mut j = vec![0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 8, 0xFF, 0xFF, 0xFF, 0xFF, 3];
    for id in 1..=3u8 {
        j.extend_from_slice(&[id, 0x11, 0]);
    }
    j.extend_from_slice(&[0xFF, 0xD9]);
    let e = mb3d::image::decode(&j).unwrap_err();
    assert!(e.contains("too large"), "{e}");
}

#[test]
fn a_jpeg_of_endless_scans_ends() {
    // a progressive 8 x 8 picture and then nothing but DC refinement scans:
    // each one walks the blocks, a file of such headers kept the reader busy
    let mut j = vec![0xFF, 0xD8, 0xFF, 0xC2, 0x00, 0x0B, 8, 0, 8, 0, 8, 1, 1, 0x11, 0];
    for _ in 0..5000 {
        j.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x08, 1, 1, 0x00, 0, 0, 0x10]);
    }
    j.extend_from_slice(&[0xFF, 0xD9]);
    let e = mb3d::image::decode(&j).unwrap_err();
    assert!(e.contains("too many scans"), "{e}");
}

#[test]
fn a_mandelbulber_section_number_cannot_ask_for_gigabytes() {
    let text = "# Mandelbulber settings file\n[main_parameters]\nimage_width 640;\n[fractal_4000000000]\nformula 1;\n[fractal_2]\nformula 2;\n";
    let s = mb3d::mandelbulber::parse(text).unwrap();
    assert_eq!(s.fractals.len(), 2);
    assert_eq!(s.fractals[1].keys.get("formula").map(String::as_str), Some("2"));
}

#[test]
fn a_swarm_job_with_impossible_numbers_is_no_job() {
    let swarm = temp("swarm");
    let job = |id: &str, text: &str| {
        let d = swarm.join("jobs").join(id);
        std::fs::create_dir_all(d.join("in")).unwrap();
        std::fs::create_dir_all(d.join("out")).unwrap();
        std::fs::write(d.join("job.txt"), text).unwrap();
    };
    job("1-good", "kind = tiles\nname = good\ncols = 2\nrows = 2\nunits = 4\nwidth = 64\nheight = 48\ncreated = 1\n");
    // no columns (a division by zero in the preview), 10^11 units (a list
    // that cannot be held), a picture 10^9 wide
    job("2-cols", "kind = tiles\nname = cols\ncols = 0\nrows = 2\nunits = 4\nwidth = 64\nheight = 48\ncreated = 2\n");
    job("3-units", "kind = montecarlo\nname = units\nunits = 99999999999\ncreated = 3\n");
    job("4-width", "kind = tiles\nname = width\ncols = 1\nrows = 1\nunits = 1\nwidth = 1000000000\nheight = 48\ncreated = 4\n");
    let jobs = mb3d::swarm::jobs(&swarm);
    let names: Vec<&str> = jobs.iter().map(|j| j.name.as_str()).collect();
    assert_eq!(names, ["good"]);
    let _ = std::fs::remove_dir_all(&swarm);
}

#[test]
fn a_swarm_animation_names_no_files_of_this_computer() {
    let d = temp("m3k");
    std::fs::write(d.join("k1.m3s"), mb3d::scene::Scene::preset("Integer Power").unwrap().to_text()).unwrap();
    let by_file = "width = 100\nheight = 80\n[keyframe]\nframes = 7\nfile = k1.m3s\n[keyframe]\nfile = k1.m3s\n";
    std::fs::write(d.join("anim.m3k"), by_file).unwrap();
    // a keyframe list of one's own opens its files as before
    assert!(mb3d::animfile::load(&d.join("anim.m3k")).is_ok());
    // a job's list must carry its parameters
    let e = mb3d::animfile::load_inline(&d.join("anim.m3k")).err().unwrap();
    assert!(e.contains("carry their parameters"), "{e}");
    // what the owner of a job writes (the parameters inline) reads
    let mut a = mb3d::animfile::load(&d.join("anim.m3k")).unwrap().anim;
    for k in a.keyframes.iter_mut() {
        k.source = None; // as swarm.rs does before writing a job
    }
    std::fs::write(d.join("job.m3k"), mb3d::animfile::write_m3k(&a, None)).unwrap();
    assert_eq!(mb3d::animfile::load_inline(&d.join("job.m3k")).unwrap().anim.keyframes.len(), 2);
    // a keyframe file that is a folder, not a file
    std::fs::create_dir_all(d.join("dir.m3s")).unwrap();
    std::fs::write(d.join("dir.m3k"), "width = 100\nheight = 80\n[keyframe]\nfile = dir.m3s\n[keyframe]\nfile = dir.m3s\n").unwrap();
    let e = mb3d::animfile::load(&d.join("dir.m3k")).err().unwrap();
    assert!(e.contains("not a file"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn names_on_other_computers_are_told() {
    use mb3d::maps::network_name;
    for n in [r"\\host\share\x.png", "//host/share/x.png", r"\/host\x.png", r"  \\host\x.png"] {
        assert!(network_name(n), "{n}");
    }
    for n in ["sky.png", "/home/me/sky.png", r"C:\maps\sky.png", r"maps\sky.png", "", r"\sky.png"] {
        assert!(!network_name(n), "{n}");
    }
}

#[test]
fn a_parameter_file_is_replaced_whole() {
    let d = temp("write");
    let p = d.join("a.m3p");
    mb3d::m3p::write_file(&p, b"first").unwrap();
    mb3d::m3p::write_file(&p, b"second, longer").unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), b"second, longer");
    // no temporary file is left next to it
    let names: Vec<String> = std::fs::read_dir(&d).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["a.m3p"]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o640)).unwrap();
        mb3d::m3p::write_file(&p, b"third").unwrap();
        assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o640);
        // a link is written through, as a plain write did
        let target = d.join("target.m3p");
        std::fs::write(&target, b"old").unwrap();
        let link = d.join("link.m3p");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        mb3d::m3p::write_file(&link, b"new").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_settings_block_of_junk_is_given_up_soon() {
    let s = mb3d::scene::Scene::preset("Integer Power").unwrap();
    let text = mb3d::m3p::raw_to_text(&mb3d::m3p::write(&s), "x");
    let junk: String = (0..3000).map(|i| format!("junk_key_{i} = 1;")).collect();
    let text = match text.find("{mb3d-rs:") {
        Some(i) => format!("{}{{mb3d-rs: {junk}{}", &text[..i], &text[i + 9..]),
        None => format!("{text}\r\n{{mb3d-rs: {junk}}}"),
    };
    let (raw, _) = mb3d::m3p::raw_from_bytes(text.into_bytes(), false).unwrap();
    let m = mb3d::m3p::parse(&raw).unwrap();
    let left_out = m.warnings.iter().filter(|w| w.contains("left out")).count();
    assert!((1..=201).contains(&left_out), "{left_out} lines left out");
    // the scene itself is read
    assert_eq!(m.scene.formulas.len(), s.formulas.len());
}
