use anyhow::Result;
use eframe::egui;
use hcom_dashboard::app::HcomDashboardApp;
use hcom_dashboard::source::HcomSyncWorker;
use hcom_dashboard::sync::BackgroundSync;
use std::time::Duration;

fn main() -> Result<()> {
    let (size, pos, maximized) =
        hcom_dashboard::config::load_state(&hcom_dashboard::config::hcom_dir()).start_viewport();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(size)
        .with_maximized(maximized)
        .with_min_inner_size([600.0, 400.0])
        .with_title("hcom-dashboard");
    if let Some(pos) = pos {
        viewport = viewport.with_position(pos);
    }
    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let worker = HcomSyncWorker::new_system();
    let sync = BackgroundSync::new(worker, 200, Duration::from_millis(16)).ok();

    eframe::run_native(
        "hcom-dashboard",
        native_options,
        Box::new(|cc| Ok(Box::new(HcomDashboardApp::new(cc, sync)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {:?}", e))
}
