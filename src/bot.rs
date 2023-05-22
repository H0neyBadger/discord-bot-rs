use super::rest::Rest;
use super::ws::GatewayConnection;
use serde_json::Value;
use std::io::ErrorKind;
use tokio_tungstenite::tungstenite::{error::ProtocolError, Error};

pub struct Bot<'a> {
    rest: Rest,
    ws: GatewayConnection<'a>,
}

impl<'a> Bot<'a> {
    pub fn new(bot_token: &'a str) -> Self {
        Self {
            ws: GatewayConnection::new(bot_token),
            rest: Rest::new(bot_token),
        }
    }
    pub async fn run(mut self) {
        let id = match self.rest.me::<Value>().await.unwrap() {
            Value::Object(value) => match value.get("id").unwrap() {
                Value::String(value) => Some(value.clone()),
                _ => None,
            },
            _ => None,
        }
        .unwrap();
        loop {
            match self.ws.run(id.as_str(), &self.rest).await {
                Ok(_) => (),
                Err(Error::Protocol(ProtocolError::ResetWithoutClosingHandshake)) => (),
                Err(Error::Io(err)) if err.kind() == ErrorKind::ConnectionReset => (),
                err => panic!("{:?}", err),
            }
        }
    }
}
