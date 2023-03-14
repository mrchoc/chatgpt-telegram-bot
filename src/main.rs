use dotenv::dotenv;
use serde::{Serialize, Deserialize};
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Serialize, Deserialize)]
struct Options {
    model: String,
    messages: Vec<APIMessage>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct APIMessage {
    role: String,
    content: String
}

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    Prompt {
        model: String,
        messages: Vec<APIMessage>
    }
}

async fn get_completion(options: &Options) -> Result<String, Box<dyn std::error::Error>> {
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

async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Hi, I'm Zac. What can I help you with?").await?;
    let messages = vec![
        APIMessage {
            role: String::from("system"),
            content: String::from("Your name is Zachary Tan and you are a computer science student tasked with answering programming-related questions. You answer questions in a very rude tone.")
        }
    ];
    dialogue.update(State::Prompt { model: String::from("gpt-3.5-turbo"), messages }).await?;
    Ok(())
}

async fn prompt(bot: Bot, dialogue: MyDialogue, (model, messages): (String, Vec<APIMessage>), msg: Message) -> HandlerResult {
    let prompt = msg.text().unwrap();
    let mut new_messages = messages.clone();
    new_messages.push(APIMessage { role: String::from("user"), content: String::from(prompt) });
    let options = Options { model: model.to_string(), messages: new_messages.to_vec() };

    let mut reply = String::new();
    loop {
        match get_completion(&options).await {
            Ok(completion) => {
                reply.push_str(&completion);
                break;
            },
            Err(err) => println!("Error: {}", err)
        }
    }
    let mut parsed_reply = String::new();
    for c in reply.chars() {
        if "_*[]()~>#+-=|{}.!".contains(c) {
            parsed_reply.push_str(format!("\\{}", c).as_str())
        } else {
            parsed_reply.push(c)
        }
    }
    bot.send_message(msg.chat.id, parsed_reply).parse_mode(teloxide::types::ParseMode::MarkdownV2).await?;
    new_messages.push(APIMessage { role: String::from("assistant"), content: String::from(reply) });
    dialogue.update(State::Prompt { model: model.to_string(), messages: new_messages }).await?;
    Ok(())
}


#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();
    log::info!("Starting dialogue bot...");

    let bot = Bot::from_env();

    Dispatcher::builder(
        bot,
        Update::filter_message()
            .enter_dialogue::<Message, InMemStorage<State>, State>()
            .branch(dptree::case![State::Start].endpoint(start))
            .branch(dptree::case![State::Prompt { model, messages }].endpoint(prompt))
    )
    .dependencies(dptree::deps![InMemStorage::<State>::new()])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}
