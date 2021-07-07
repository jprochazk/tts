use std::num::NonZeroU8;

/// Start the TTS proxy with the given configuration.
#[derive(Debug, Clone, clap::Clap)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
#[clap(setting = clap::AppSettings::ColorAuto)]
pub struct Config {
    /// The port to run the proxy on.
    #[clap(short, long, name = "PORT", default_value = "3031")]
    pub port: u16,
    /// The API URL to proxy requests to.
    #[clap(short, long, default_value = crate::API_URL, name = "URL")]
    pub api_url: String,
    /// The number of retry attempts the proxy should make if the API fails to respond
    // or returns a server error.
    #[clap(short, long, name = "ATTEMPTS", default_value = "3")]
    pub retry_attempts: NonZeroU8,
    /// The maximum number of seconds a request to the API may take.
    #[clap(short = 't', long = "timeout", name = "SECONDS", default_value = "180")]
    pub api_timeout_seconds: u8,
    /// The path to the directory to store the logs in.
    #[clap(short, long, name = "PATH")]
    pub log_directory: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 3031,
            api_url: crate::API_URL.into(),
            retry_attempts: NonZeroU8::new(3).unwrap(),
            api_timeout_seconds: 180,
            log_directory: None,
        }
    }
}
