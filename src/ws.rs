use super::gateway::{
    EventName, Gateway, GatewayDispatch, GatewayKind, GatewayOp, GatewayOpCode, Hello, Identify,
    Resume,
};
use super::rest::Rest;
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

pub struct GatewayConnection<'a> {
    token: &'a str,
    resume_gateway_url: Option<String>,
    session_id: Option<String>,
    sequence: u64,
}

impl<'a> GatewayConnection<'a> {
    async fn heartbeat(socket: Socket, heartbeat_interval: u64) -> Result<(), Error> {
        loop {
            sleep(Duration::from_millis(heartbeat_interval)).await;
            let mut socket = socket.lock().await;
            // println!("Send Hello (heartbeat_interval: {heartbeat_interval})");
            let seq = socket.get_seq();
            let msg = Hello::new(seq);
            socket.send(msg).await?
        }
    }

    pub fn new(token: &'a str) -> Self {
        Self {
            token: token,
            resume_gateway_url: None,
            session_id: None,
            sequence: 0_u64,
        }
    }

    pub async fn run(&mut self, id: &str, rest: &Rest) -> Result<(), Error> {
        let url = self
            .resume_gateway_url
            .as_deref()
            .unwrap_or("wss://gateway.discord.gg/");
        let res = connect_async(Url::parse(url).unwrap()).await;
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
            let dispatch: Gateway = match &msg? {
                Message::Text(txt) => {
                    // println!("Received {}", &txt);
                    serde_json::from_str(txt.as_str()).expect(format!("{:?}", &txt).as_str())
                }
                // reset bot connection loop
                Message::Close(_) => return Ok(()),
                err => panic!("Received {:?}", err),
            };

            if let Some(sequence) = dispatch.sequence {
                // save last sequence number
                self.sequence = sequence;
            };

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
                        Self::heartbeat(heartbeat_write, heartbeat_interval).await?;
                        Ok::<(), Error>(())
                    });
                    // auth
                    if self.resume_gateway_url.is_none() {
                        write_send!(Identify::new(self.token));
                    } else {
                        write_send!(Resume::new(
                            self.token,
                            self.session_id.as_deref().unwrap(),
                            self.sequence
                        ));
                    }
                }
                GatewayKind::GatewayOp(GatewayOp {
                    op: GatewayOpCode::Reconnect,
                    ..
                }) => {
                    write_send!(Resume::new(
                        self.token,
                        self.session_id.as_deref().unwrap(),
                        self.sequence
                    ))
                }
                GatewayKind::GatewayOp(GatewayOp {
                    op: GatewayOpCode::HeartbackACK,
                    ..
                }) => {}
                GatewayKind::Event(GatewayDispatch {
                    data: EventName::Ready(data),
                    ..
                }) => {
                    println!(
                        "Ready resume_gateway_url {}, session_id {}",
                        data.resume_gateway_url.as_str(),
                        data.session_id.as_str()
                    );
                    self.resume_gateway_url = Some(data.resume_gateway_url.to_owned());
                    self.session_id = Some(data.session_id.to_owned());
                }
                GatewayKind::Event(GatewayDispatch {
                    data: EventName::MessageCreate(data),
                    ..
                }) => {
                    // println!("MessageCreated {}", json!(data));
                    if data.mentions.iter().find(|user| user.id == id).is_some() {
                        // react to mention only
                        rest.react(data.channel_id.as_str(), data.id.as_str(), "🔥")
                            .await
                            .unwrap();
                    }
                }
                GatewayKind::Event(GatewayDispatch { data: event, .. }) => {
                    println!("Event {:?}", event);
                }
                _ => panic!("Code {:?}", dispatch),
            };
        }
        Ok(())
    }
}
