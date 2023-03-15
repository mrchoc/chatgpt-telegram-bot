use dotenv::dotenv;
use teloxide::{dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler}, prelude::*,utils::command::BotCommands};

mod api;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    Prompt {
        model: String,
        messages: Vec<api::APIMessage>
    }
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "start/restart bot conversation.")]
    Start,
}

async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Hi, I'm Zac. What can I help you with?").await?;
    let messages = vec![
        api::APIMessage {
            role: String::from("system"),
            content: String::from("Your name is Zachary Tan and you are a computer science student tasked with answering programming-related questions. You answer questions in a very rude tone.")
        }
    ];
    dialogue.update(State::Prompt { model: String::from("gpt-3.5-turbo"), messages }).await?;
    Ok(())
}

async fn prompt(bot: Bot, dialogue: MyDialogue, (model, messages): (String, Vec<api::APIMessage>), msg: Message) -> HandlerResult {
    let prompt = msg.text().unwrap();
    let mut new_messages = messages.clone();
    new_messages.push(api::APIMessage { role: String::from("user"), content: String::from(prompt) });
    let options = api::Options { model: model.to_string(), messages: new_messages.to_vec() };

    let mut reply = String::new();
    loop {
        match api::get_completion(&options).await {
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
    new_messages.push(api::APIMessage { role: String::from("assistant"), content: String::from(reply) });
    dialogue.update(State::Prompt { model: model.to_string(), messages: new_messages }).await?;
    Ok(())
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Start].endpoint(start));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::Prompt { model, messages }].endpoint(prompt));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
}


#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();
    log::info!("Starting dialogue bot...");

    let bot = Bot::from_env();

    Dispatcher::builder(bot, schema())
    .dependencies(dptree::deps![InMemStorage::<State>::new()])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}
