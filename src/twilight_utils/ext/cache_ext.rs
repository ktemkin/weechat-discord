use crate::twilight_utils::dynamic_channel::DynamicChannel;
use twilight_cache_inmemory::InMemoryCache;
use twilight_model::id::ChannelId;

pub trait CacheExt {
    fn dynamic_channel(&self, channel_id: ChannelId) -> Option<DynamicChannel>;
}

impl CacheExt for InMemoryCache {
    fn dynamic_channel(&self, channel_id: ChannelId) -> Option<DynamicChannel> {
        if let Some(channel) = self.guild_channel(channel_id) {
            return Some(DynamicChannel::Guild(channel.resource().clone()));
        }
        if let Some(channel) = self.private_channel(channel_id) {
            return Some(DynamicChannel::Private(channel.value().clone()));
        }
        if let Some(channel) = self.group(channel_id) {
            return Some(DynamicChannel::Group(channel.value().clone()));
        }

        None
    }
}
