pub mod logic;

use espionox::telemetry::{get_subscriber, init_subscriber};
use logic::*;
use once_cell::sync::Lazy;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "gui".to_string();
    // if std::env::var("EGUI_LOG").is_ok() {
    let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
    init_subscriber(subscriber);
    // } else {
    //     let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
    //     init_subscriber(subscriber);
    // }
});

#[tokio::main]
async fn main() {
    Lazy::force(&TRACING);
    MainApplication::run().expect("Failed to run ap");
}
