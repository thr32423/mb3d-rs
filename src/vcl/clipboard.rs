//! Text clipboard (the system clipboard; a process-local one when it is
//! not available, e.g. in tests).

use std::cell::RefCell;

thread_local! {
    static CB: RefCell<Option<arboard::Clipboard>> = const { RefCell::new(None) };
    static LOCAL: RefCell<String> = const { RefCell::new(String::new()) };
}

fn with<R>(f: impl FnOnce(&mut arboard::Clipboard) -> Option<R>) -> Option<R> {
    CB.with(|cb| {
        let mut cb = cb.borrow_mut();
        if cb.is_none() {
            *cb = arboard::Clipboard::new().ok();
        }
        cb.as_mut().and_then(f)
    })
}

pub fn get() -> Option<String> {
    with(|c| c.get_text().ok()).or_else(|| LOCAL.with(|l| Some(l.borrow().clone())).filter(|s| !s.is_empty()))
}

pub fn set(s: &str) {
    LOCAL.with(|l| *l.borrow_mut() = s.to_string());
    with(|c| c.set_text(s.to_string()).ok());
}
