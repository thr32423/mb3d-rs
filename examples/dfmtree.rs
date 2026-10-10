//! Prints the control tree of a form file (name, class, caption, events).
fn main() {
    let p = std::env::args().nth(1).expect("dfm file");
    let t = std::fs::read_to_string(&p).unwrap();
    let o = mb3d::vcl::dfm::parse(&t).unwrap();
    fn walk(o: &mb3d::vcl::dfm::Obj, d: usize) {
        let mut s = format!("{}{} {}", "  ".repeat(d), o.name, o.class.trim_start_matches('T'));
        for k in ["Caption", "Text", "Tag"] {
            if let Some(v) = o.get(k) {
                let v = match v { mb3d::vcl::dfm::Value::Str(s) => format!("'{}'", s.replace("\r\n", "|")), v => format!("{v:?}") };
                s += &format!(" {k}={}", v.chars().take(40).collect::<String>());
            }
        }
        if o.bool("Visible") == Some(false) { s += " HIDDEN"; }
        if o.bool("Enabled") == Some(false) { s += " DISABLED"; }
        for (k, v) in &o.props {
            if k.starts_with("On") { s += &format!(" {k}:{}", v.as_str().unwrap_or("")); }
            if k == "Items.Strings" { s += &format!(" items={:?}", v.as_strings()); }
            if k == "PopupMenu" { s += &format!(" popup={}", v.as_str().unwrap_or("")); }
        }
        println!("{s}");
        for c in &o.children { walk(c, d + 1); }
    }
    walk(&o, 0);
}
