use tts_proxy::run;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    run(tts_proxy::config::Config::default())?.await
}
