use std::sync::LazyLock;

static SHARED_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent("Kobup/1.0")
        .build()
        .expect("failed to build HTTP client")
});

pub fn client() -> reqwest::Client {
    SHARED_CLIENT.clone()
}
