use tracing::Level;
use tracing_subscriber::{
    fmt::{format::Writer, time::FormatTime},
    EnvFilter,
};

pub fn set_log(filter: &str) {
    struct LocalTimer;
    impl FormatTime for LocalTimer {
        fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
            write!(w, "{}", chrono::Local::now().format("%m-%d %T%.3f"))
        }
    }
    let filter = EnvFilter::try_new(filter).unwrap();
    // tracing 初始化
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_timer(LocalTimer)
        .with_thread_ids(true)
        .with_env_filter(filter)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
