use std::net::TcpListener;

use tts_proxy::run;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let address = format!("127.0.0.1:{}", 8080);
    let listener = TcpListener::bind(address)?;
    run(tts_proxy::config::Config::default(), listener)?.await
}
