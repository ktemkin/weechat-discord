use serde::Serialize;
use std::collections::HashMap;
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker},
    Id,
};

#[derive(Serialize, Debug)]
pub struct GuildSubscriptionFull {
    pub guild_id: Id<GuildMarker>,
    pub typing: bool,
    pub activities: bool,
    pub threads: bool,
    pub channels: HashMap<Id<ChannelMarker>, Vec<Vec<u8>>>,
}

#[derive(Serialize, Debug)]
pub struct GuildSubscriptionMinimal {
    pub guild_id: Id<GuildMarker>,
    pub channels: HashMap<Id<ChannelMarker>, Vec<Vec<u8>>>,
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
    pub fn into_message(self) -> twilight_gateway::shard::raw_message::Message {
        twilight_gateway::shard::raw_message::Message::Text(
            serde_json::to_string(&self).expect("valid serde"),
        )
    }
}
