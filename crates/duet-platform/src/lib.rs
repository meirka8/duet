use std::cell::Cell;

thread_local! {
    static UI_THREAD: Cell<bool> = const { Cell::new(false) };
}

/// Sets whether the current thread is the UI thread.
pub fn set_ui_thread(is_ui: bool) {
    UI_THREAD.with(|cell| cell.set(is_ui));
}

/// Asserts that the current thread is not the UI thread.
/// Panics in debug builds if called from the UI thread.
pub fn assert_not_ui_thread() {
    #[cfg(debug_assertions)]
    {
        UI_THREAD.with(|cell| {
            if cell.get() {
                panic!("Blocking operation called on the UI thread!");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "Blocking operation called on the UI thread!")]
    fn test_ui_thread_guard_panics_on_ui_thread() {
        set_ui_thread(true);
        assert_not_ui_thread();
    }

    #[test]
    fn test_ui_thread_guard_does_not_panic_on_non_ui_thread() {
        set_ui_thread(false);
        assert_not_ui_thread();
    }
}
