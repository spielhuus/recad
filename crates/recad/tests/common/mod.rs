use std::sync::Once;

static INIT: Once = Once::new();

pub fn setup() {
    INIT.call_once(|| {
        spdlog::default_logger().set_level_filter(spdlog::LevelFilter::All);
    });
}
