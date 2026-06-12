#[cfg(feature = "gpui-ui")]
fn main() {
    let _telemetry_guard = notatus_telemetry::init_telemetry();

    tracing::info!("starting notatus-ui");

    notatus_ui::launch_gpui();
}

#[cfg(not(feature = "gpui-ui"))]
fn main() {
    let _telemetry_guard = notatus_telemetry::init_telemetry();

    tracing::error!("notatus-ui was built without the gpui-ui feature. Run with default features enabled: cargo run -p notatus-ui");
}
