use serde_json::Value;
use std::env;
use tokio;

mod bot;
mod gateway;
mod ws;

#[tokio::main]
async fn main() {
    let bot_token = env::var("BOT_TOKEN").expect("BOT_TOKEN not set");
    let bot = bot::Bot::new(bot_token.as_str());
    let ret: Value = bot.me().await.unwrap();
    ws::GatewayConnection::run(bot_token.as_str(), &bot).await;
    dbg!(&ret);
}
