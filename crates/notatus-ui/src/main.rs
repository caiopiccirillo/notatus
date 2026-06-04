#[cfg(feature = "gpui-ui")]
fn main() {
    notatus_ui::launch_gpui();
}

#[cfg(not(feature = "gpui-ui"))]
fn main() {
    eprintln!("notatus-ui was built without the gpui-ui feature.");
    eprintln!("Run with default features enabled: cargo run -p notatus-ui");
}
