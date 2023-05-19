mod app;
mod simulation;
mod utils;

use fern::colors::ColoredLevelConfig;
fn main() {
    let color_config = ColoredLevelConfig::new();
    fern::Dispatch::new()
        .level(log::LevelFilter::Debug)
        .level_for("wgpu_hal", log::LevelFilter::Warn)
        .level_for("naga", log::LevelFilter::Warn)
        .level_for("wgpu_core", log::LevelFilter::Warn)
        .format(|out, message, record| {
            out.finish(format_args!(
                "{time}[{level}][{target}] {message}",
                time = chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                level = record.level(),
                target = record.target(),
                message = message,
            ))
        })
        .chain(fern::Dispatch::new().chain(std::io::stdout()).format(move |out, message, record| {
            out.finish(format_args!(
                "{color}{message}{color_reset}",
                message = message,
                color = format_args!("\x1B[{color_number}m", color_number = color_config.get_color(&record.level()).to_fg_str()),
                color_reset = "\x1B[0m",
            ))
        }))
        .apply()
        .unwrap();

    oxyde::run_application::<app::RustyBoids>(
        oxyde::AppConfig {
            is_resizable: true,
            title: "Rusty Boids",
            control_flow: oxyde::winit::event_loop::ControlFlow::Poll,
            ..oxyde::AppConfig::default()
        },
        oxyde::RenderingConfig {
            power_preference: oxyde::wgpu::PowerPreference::HighPerformance,
            // window_surface_present_mode: oxyde::wgpu::PresentMode::Immediate,
            ..oxyde::RenderingConfig {
                device_features: wgpu_profiler::GpuProfiler::ALL_WGPU_TIMER_FEATURES | oxyde::wgpu::Features::default(),
                device_limits: oxyde::wgpu::Limits{
                    max_bind_groups: 6,
                    ..oxyde::wgpu::Limits::default()
                },
                ..oxyde::RenderingConfig::default()
            }
        },
    )
    .unwrap();
}
