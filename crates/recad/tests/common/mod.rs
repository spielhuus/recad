use std::sync::Once;

static INIT: Once = Once::new();

pub fn setup() {
    INIT.call_once(|| {
        match spdlog::init_log_crate_proxy() {
            Ok(_) => {
                spdlog::info!("Proxy initialized successfully.");
                spdlog::default_logger().set_level_filter(spdlog::LevelFilter::Off);
            }
            Err(e) => {
                // Log this to the file directly using spdlog, so we see the error!
                spdlog::error!(
                    "FAILED to init log proxy: {}. `log::info!` will NOT work.",
                    e
                );
            }
        }
    });
}
