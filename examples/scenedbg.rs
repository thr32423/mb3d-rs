fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[1]));
    let m = mb3d::m3p::load(std::path::Path::new(&a[0])).unwrap();
    println!("{:#?}", m.scene);
}
