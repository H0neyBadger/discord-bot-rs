use super::bot::Bot;
use super::gateway::{
    EventName, Gateway, GatewayDispatch, GatewayKind, GatewayOp, GatewayOpCode, Hello, Identify,
};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::{
    net::TcpStream,
    sync::Mutex,
    time::{sleep, Duration},
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error, Message},
    MaybeTlsStream, WebSocketStream,
};
use url::Url;

struct SocketSequence {
    socket: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    sequence: u64,
}

impl SocketSequence {
    pub fn get_seq(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }

    pub async fn send<T>(&mut self, msg: T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let txt = format!("{}", json!(msg));
        let msg = Message::Text(txt.into());
        self.socket.send(msg).await
    }

    pub fn new(socket: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>) -> Self {
        Self {
            sequence: 0,
            socket: socket,
        }
    }
}

type Socket = Arc<Mutex<SocketSequence>>;

#[derive(Debug)]
pub struct GatewayConnection {}

impl GatewayConnection {
    async fn heartbeat(socket: Socket, heartbeat_interval: u64) -> Message {
        loop {
            sleep(Duration::from_millis(heartbeat_interval)).await;
            let mut socket = socket.lock().await;
            println!("Send Hello (heartbeat_interval: {heartbeat_interval})");
            let seq = socket.get_seq();
            let msg = Hello::new(seq);
            socket.send(msg).await.unwrap();
        }
    }

    pub async fn run(token: &str, bot: &Bot) {
        let res = connect_async(Url::parse("wss://gateway.discord.gg/").unwrap()).await;
        let (socket, response) = match res {
            Ok(values) => values,
            Err(Error::Http(response)) if response.body().is_some() => panic!(
                "{:?}",
                std::str::from_utf8(response.body().as_ref().unwrap())
            ),
            Err(err) => panic!("{:?}", err),
        };
        println!("Connected to the server");
        println!("Response HTTP code: {}", response.status());
        println!("Response contains the following headers:");
        for (ref header, _value) in response.headers() {
            println!("* {}", header);
        }
        let (write, mut read) = socket.split();

        let write: Socket = Arc::new(Mutex::new(SocketSequence::new(write)));

        #[macro_export]
        macro_rules! write_send {
            ($x:expr) => {
                write.lock().await.send($x).await.unwrap()
            };
        }

        while let Some(msg) = read.next().await {
            let dispatch: Gateway = match &msg {
                Ok(Message::Text(txt)) => {
                    serde_json::from_str(txt.as_str()).expect(format!("{:?}", &txt).as_str())
                }
                _ => panic!("Received {:?}", msg),
            };
            println!("Received {}", json!(dispatch));
            let kind = &dispatch.kind;
            match kind {
                GatewayKind::GatewayOp(GatewayOp {
                    op: GatewayOpCode::Hello,
                    data: Value::Object(v),
                    ..
                }) => {
                    let heartbeat_interval = v.get("heartbeat_interval").unwrap().as_u64().unwrap();
                    let heartbeat_write = write.clone();
                    tokio::spawn(async move {
                        // send heartbeat loop in background
                        Self::heartbeat(heartbeat_write, heartbeat_interval).await;
                    });
                    // auth
                    write_send!(Identify::new(token))
                }
                GatewayKind::GatewayOp(GatewayOp {
                    op: GatewayOpCode::HeartbackACK,
                    ..
                }) => {}
                GatewayKind::Event(GatewayDispatch {
                    data: EventName::Ready(_),
                    ..
                }) => {
                    println!("Ready");
                }
                GatewayKind::Event(GatewayDispatch {
                    data: EventName::MessageCreate(data),
                    ..
                }) => {
                    println!("MessageCreated {}", json!(data));
                    bot.react(data.channel_id.as_str(), data.id.as_str(), "🔥")
                        .await
                        .unwrap();
                    // bot.react(data.channel_id.as_str(), data.id.as_str(), "738480321217167460").await.unwrap();
                }
                GatewayKind::Event(GatewayDispatch { data: event, .. }) => {
                    println!("Event {:?}", event);
                }
                _ => panic!("Code {:?}", dispatch),
            };
        }
    }
}
