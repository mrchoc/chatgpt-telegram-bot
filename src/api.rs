use dotenv::dotenv;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Options {
    pub model: String,
    pub messages: Vec<APIMessage>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct APIMessage {
    pub role: String,
    pub content: String
}

pub async fn get_completion(options: &Options) -> Result<String, Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_token = std::env::var("OPENAI_SK")?;

    let client = reqwest::Client::new();

    let response: serde_json::Value = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {api_token}"))
        .json(&options)
        .send()
        .await?
        .json()
        .await?;

    // let json: &str = serde_json::from_str(&text).expect("deserialize").to_string();
    // let completion = format!("{}", choices[0].get("text").unwrap());
    // println!("{:?}\n", options);
    match response.get("choices") {
        Some(choices) => {
            let message = choices[0]["message"]["content"].as_str().unwrap();
            Ok(format!("{}", message))
        }
        None => Err(format!("{}", response["error"]["message"].as_str().unwrap()))?
    }
}
