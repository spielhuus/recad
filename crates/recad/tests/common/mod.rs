use std::sync::Once;

static INIT: Once = Once::new();

pub fn setup() {
    INIT.call_once(|| {
        spdlog::default_logger().set_level_filter(spdlog::LevelFilter::All);
        //     match spdlog::init_log_crate_proxy() {
        //         Ok(_) => {
        //             spdlog::default_logger().set_level_filter(spdlog::LevelFilter::All);
        //             spdlog::info!("Proxy initialized successfully.");
        //         }
        //         Err(e) => {
        //             // Log this to the file directly using spdlog, so we see the error!
        //             spdlog::error!(
        //                 "FAILED to init log proxy: {}. `log::info!` will NOT work.",
        //                 e
        //             );
        //         }
        //     }
    });
}
