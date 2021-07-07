mod common;

#[actix_rt::test]
async fn test_health_check() {
    let addr = common::spawn_app().unwrap();

    let response = reqwest::Client::new()
        .get(format!("{}/health_check", addr))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());
}
