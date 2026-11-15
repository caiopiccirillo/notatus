//! Helpers shared by the window tests.

use crate::app::*;
use gpui::{TestAppContext, WindowHandle, WindowOptions};

/// Opens a Notatus window the way the app does and returns its handle and
/// view, after the first layout.
pub(in crate::app) fn open_test_window(
    cx: &mut TestAppContext,
) -> (WindowHandle<Root>, gpui::Entity<NotatusWindow>) {
    let view_slot = Rc::new(RefCell::new(None));
    let slot = view_slot.clone();
    let window = cx.update(|cx| {
        gpui_component::init(cx);
        actions::bind_keys(cx);
        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|cx| NotatusWindow::new(window, cx));
            *slot.borrow_mut() = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx))
        })
        .unwrap()
    });
    cx.run_until_parked();
    let view = view_slot.borrow_mut().take().expect("window view");
    (window, view)
}
