use anyhow::Result;
use eframe::egui;
use hcom_dashboard::app::HcomDashboardApp;
use hcom_dashboard::source::HcomSyncWorker;
use hcom_dashboard::sync::BackgroundSync;
use std::time::Duration;

fn main() -> Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("hcom-dashboard"),
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
