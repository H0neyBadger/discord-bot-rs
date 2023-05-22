use std::env;
use tokio;

mod bot;
mod gateway;
mod rest;
mod ws;

#[tokio::main]
async fn main() {
    let bot_token = env::var("BOT_TOKEN").expect("BOT_TOKEN not set");
    let bot = bot::Bot::new(bot_token.as_str());
    bot.run().await;
}
