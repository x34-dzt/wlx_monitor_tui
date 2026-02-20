mod app;
mod config;
mod setup;
mod ui;

use std::env;
use std::sync::mpsc::{self, Receiver};

use color_eyre::eyre::Result;
use wlx_monitors::{WlMonitorEvent, WlMonitorManager};
use xwlm_cfg::Compositor;

use crate::{app::App, config::AppConfig};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("xwlm {}", VERSION);
        return Ok(());
    }

    color_eyre::install()?;

    let compositor = xwlm_cfg::detect();

    let app_config = match load_config(compositor)? {
        Some(cfg) => cfg,
        None => return Ok(()),
    };

    let (emitter, event_receiver) = mpsc::sync_channel(16);
    let (controller, action_receiver) = mpsc::sync_channel(16);

    let (state, event_queue) =
        WlMonitorManager::new_connection(emitter, action_receiver)
            .expect("Failed to connect to Wayland");

    std::thread::spawn(move || {
        state.run(event_queue).expect("Event loop error");
    });

    let app = app::App::new(
        controller,
        compositor,
        app_config.monitor_config_path,
        app_config.workspace_count,
    );
    run_xwlm(app, event_receiver)
}

fn load_config(compositor: Compositor) -> Result<Option<AppConfig>> {
    match config::load()? {
        Some(cfg) => {
            if !config::monitor_config_exists(&cfg.monitor_config_path) {
                eprintln!(
                    "Monitor config file not found: {}",
                    cfg.monitor_config_path
                );
                eprintln!("Re-running setup...");
                return run_setup_and_save(compositor);
            }
            Ok(Some(cfg))
        }
        None => run_setup_and_save(compositor),
    }
}

fn run_setup_and_save(compositor: Compositor) -> Result<Option<AppConfig>> {
    let result = run_setup(compositor);
    match result? {
        Some(cfg) => {
            config::save(&cfg)?;
            Ok(Some(cfg))
        }
        None => Ok(None),
    }
}

fn run_setup(compositor: Compositor) -> Result<Option<AppConfig>> {
    let terminal = ratatui::init();
    let result = setup::run(terminal, compositor);
    ratatui::restore();
    result
}

fn run_xwlm(mut app: App, event_rx: Receiver<WlMonitorEvent>) -> Result<()> {
    let terminal = ratatui::init();
    let result = ui::run(terminal, &mut app, event_rx);
    ratatui::restore();
    result
}
