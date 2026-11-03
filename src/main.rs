use eframe::egui;
fn main() -> eframe::Result {
    eframe::run_native(
        "Conway Topograph",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1360.0, 980.0]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(conway_topograph::app::App::new(cc)))),
    )
}
