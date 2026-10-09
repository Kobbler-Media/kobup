use reqwest::Client;

pub fn client() -> Result<Client, Box<dyn std::error::Error>> {
    Ok(Client::builder().user_agent("kobup-cli/0.1.0").build()?)
}

pub fn auth_header(token: &str) -> String {
    format!("Bearer {token}")
}
