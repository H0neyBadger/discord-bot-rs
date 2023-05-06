use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_repr::*;

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
pub enum GatewayDispatchOpCode {
    Dispatch = 0, // dispatches an event
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
pub enum GatewayOpCode {
    Heartbeat = 1,           // used for ping checking
    Identify = 2,            // used for client handshake
    StatusUpdate = 3,        // used to update the client status
    VoiceStateUpdate = 4,    // used to join/move/leave voice channels
    VoiceServerPing = 5,     // used for voice ping checking
    Resume = 6,              // used to resume a closed connection
    Reconnect = 7,           // used to tell clients to reconnect to the gateway
    RequestGuildMembers = 8, // used to request guild members
    InvalidSession = 9,      // used to notify client they have an invalid session id
    Hello = 10, // sent immediately after connecting, contains heartbeat and server debug information
    HeartbackACK = 11, // sent immediately following a client heartbeat that was received
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Hello {
    op: GatewayOpCode,
    #[serde(rename = "d")]
    data: u64,
}

impl Hello {
    pub fn new(seq: u64) -> Self {
        Self {
            op: GatewayOpCode::Heartbeat,
            data: seq,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct IdetifyProperties {
    os: String,
    browser: String,
    device: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct IdetifyPresenceActivity {
    name: String, // Activity's name
    #[serde(rename = "type")]
    _type: u32, // integer Activity type
    url: Option<String>, // Stream URL, is validated when type is 1
    created_at: u64, // integer Unix timestamp (in milliseconds) of when the activity was added to the user's session
    timestamps: Option<u64>, // timestamps object   Unix timestamps for start and/or end of the game
    application_id: Option<String>, // snowflake   Application ID for the game
    details: Option<String>, // What the player is currently doing
    state: Option<String>, // User's current party status
    emoji: Option<String>, // emoji object   Emoji used for a custom status
    // party:  Option<IdetifyPresenceActivityParty>, // party object    Information for the current party of the player
    // assets: Option< assets object   Images for the presence and their hover texts
    // secrets: Option<    secrets object  Secrets for Rich Presence joining and spectating
    instance: Option<bool>, // boolean Whether or not the activity is an instanced game session
    flags: Option<u32>, // integer Activity flags ORd together, describes what the payload includes
                        // buttons: Option<    array of buttons    Custom buttons shown in the Rich Presence (max 2)
}

#[derive(Serialize, Deserialize, Debug)]
struct IdetifyPresence {
    since: Option<u64>, // Unix time (in milliseconds) of when the client went idle, or null if the client is not idle
    activities: Vec<IdetifyPresenceActivity>, //  array of activity objects   User's activities
    status: String,     // string  User's new status
    afk: bool,          // boolean Whether or not the client is afk
}

#[derive(Serialize, Deserialize, Debug)]
struct IdentifyData {
    token: String,                     // Authentication token
    properties: IdetifyProperties,     // Connection properties
    compress: Option<String>,          // Whether this connection supports compression of packets
    large_threshold: Option<u8>, //Value between 50 and 250, total number of members where the gateway will stop sending offline members in the guild member list
    shard: [u32; 2],             // Used for Guild Sharding
    presence: Option<IdetifyPresence>, // update presence object  Presence structure for initial presence information -
    intents: u32,                      // integer Gateway Intents you wish to receive
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Identify {
    op: GatewayOpCode,
    #[serde(rename = "d")]
    data: IdentifyData,
}

impl Identify {
    pub fn new(token: &str) -> Self {
        Self {
            op: GatewayOpCode::Identify,
            data: IdentifyData {
                token: String::from(token),
                properties: IdetifyProperties {
                    os: String::from("linux"),
                    browser: String::from("Mario"),
                    device: String::from("My computer"),
                },
                compress: None,
                large_threshold: Some(250),
                shard: [0, 1],
                presence: None,
                intents: 4608,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageCreateAuthor {
    pub username: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageCreate {
    pub author: MessageCreateAuthor,
    pub channel_id: String,
    pub id: String,
    pub content: String,
    pub guild_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "t", content = "d")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventName {
    Ready(Value),
    ChannelCreate(Value),
    // https://discord.com/developers/docs/topics/gateway#gateway-events
    MessageCreate(MessageCreate),
    MessageUpdate(Value),
    MessageDelete(Value),
    MessageDeleteBulk(Value),
    ChannelPinsUpdate(Value),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GatewayDispatch {
    pub op: GatewayDispatchOpCode,
    #[serde(flatten)]
    pub data: EventName,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GatewayOp {
    pub op: GatewayOpCode,
    #[serde(rename = "d")]
    pub data: Value,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum GatewayKind {
    GatewayOp(GatewayOp),
    Event(GatewayDispatch),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Gateway {
    #[serde(flatten)]
    pub kind: GatewayKind,
    #[serde(rename = "s")]
    pub sequence: Option<u64>,
}
