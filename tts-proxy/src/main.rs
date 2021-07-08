use clap::Clap;
use tts_proxy::start;

fn main() -> std::io::Result<()> {
    let config = tts_proxy::config::Config::parse();

    if cfg!(target_family = "unix") && config.daemonize {
        match fork::daemon(false, true) {
            Ok(fork::Fork::Child) => (), // We're in the granchild process, do nothing
            Ok(_) => {
                // We're in the intermediate child process, exit
                std::process::exit(0);
            }
            Err(_) => {
                panic!("Failed to daemonize the process.");
            }
        }
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move { actual_main(config).await })
}

async fn actual_main(mut config: tts_proxy::config::Config) -> std::io::Result<()> {
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
