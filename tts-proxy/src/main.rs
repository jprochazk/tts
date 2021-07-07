use clap::Clap;
use tts_proxy::start;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut config = tts_proxy::config::Config::parse();

    if let Some(directory) = config.log_directory.take() {
        let file_appender = tracing_appender::rolling::never(directory, "tts-proxy.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_owned());
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(non_blocking)
            .init();
    } else {
        let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "none".to_owned());
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    start(config).await;
    Ok(())
}
