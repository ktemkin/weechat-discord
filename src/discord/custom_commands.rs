use serde::Serialize;
use std::collections::HashMap;
use twilight_model::id::{ChannelId, GuildId};

#[derive(Serialize, Debug)]
pub struct GuildSubscriptionFull {
    pub guild_id: GuildId,
    pub typing: bool,
    pub activities: bool,
    pub threads: bool,
    pub channels: HashMap<ChannelId, Vec<Vec<u8>>>,
}

#[derive(Serialize, Debug)]
pub struct GuildSubscriptionMinimal {
    pub guild_id: GuildId,
    pub channels: HashMap<ChannelId, Vec<Vec<u8>>>,
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum GuildSubscriptionInfo {
    Full(GuildSubscriptionFull),
    Minimal(GuildSubscriptionMinimal),
}

#[derive(Serialize, Debug)]
pub struct GuildSubscription {
    pub d: GuildSubscriptionInfo,
    pub op: u8,
}

impl GuildSubscription {
    pub fn as_message(self) -> twilight_gateway::shard::raw_message::Message {
        twilight_gateway::shard::raw_message::Message::Text(
            serde_json::to_string(&self).expect("valid serde"),
        )
    }
}
