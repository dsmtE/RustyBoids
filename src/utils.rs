use oxyde::egui;

pub fn setup_ui_profiler(ui: &mut egui::Ui, profiling_data: &[wgpu_profiler::GpuTimerScopeResult], levels_default_open: i32) {
    for scope in profiling_data.iter() {
        let time = format!("{:.3}ms", (scope.time.end - scope.time.start) * 1000.0);
        if scope.nested_scopes.is_empty() {
            ui.horizontal(|ui| {
                ui.label(&scope.label);
                ui.with_layout(egui::Layout::default().with_cross_align(egui::Align::Min), |ui| {
                    ui.label(time);
                });
            });
        } else {
            egui::CollapsingHeader::new(format!("{}  -  {}", scope.label, time))
                .id_source(&scope.label)
                .default_open(levels_default_open > 0)
                .show(ui, |ui| setup_ui_profiler(ui, &scope.nested_scopes, levels_default_open - 1));
        }
        ui.end_row();
    }
}
