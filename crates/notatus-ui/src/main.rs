#[cfg(feature = "gpui-ui")]
fn main() {
    notatus_ui::launch_gpui();
}

#[cfg(not(feature = "gpui-ui"))]
fn main() {
    eprintln!("notatus-ui was built without the gpui-ui feature.");
    eprintln!("Run with: cargo run -p notatus-ui --features gpui-ui");
}
