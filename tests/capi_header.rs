//! The header and the functions say the same: every function of
//! capi/include/mb3d.h is defined in capi/src/lib.rs, and every C function defined
//! there is in the header (a caller of one missing in the library fails
//! only when it links; one missing in the header cannot be called).

use std::collections::BTreeSet;

/// The names starting `mb3d_` that are followed by `(` after `before`.
fn names(text: &str, before: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for (at, _) in text.match_indices("mb3d_") {
        let name: String = text[at..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        let word_start = !text[..at].ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_');
        if word_start && text[at + name.len()..].starts_with('(') && text[..at].ends_with(before) {
            found.insert(name);
        }
    }
    found
}

#[test]
fn header_and_functions_agree() {
    let header = include_str!("../capi/include/mb3d.h");
    let code = include_str!("../capi/src/lib.rs");
    // a declaration: the name after a type (a space or a pointer's star)
    let declared: BTreeSet<String> = names(header, " ").union(&names(header, "*")).cloned().collect();
    let defined = names(code, "extern \"C\" fn ");
    assert!(declared.len() > 40, "the header's functions were not found: {declared:?}");
    let only_header: Vec<_> = declared.difference(&defined).collect();
    let only_code: Vec<_> = defined.difference(&declared).collect();
    assert!(only_header.is_empty(), "in mb3d.h but not defined in capi/src/lib.rs: {only_header:?}");
    assert!(only_code.is_empty(), "defined in capi/src/lib.rs but not in mb3d.h: {only_code:?}");
}
