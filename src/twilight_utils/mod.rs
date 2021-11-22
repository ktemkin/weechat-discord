use crate::{twilight_utils::ext::GuildChannelExt, utils};
use twilight_cache_inmemory::{model::CachedGuild, InMemoryCache};
use twilight_model::{channel::GuildChannel, id::GuildId};

mod color;
pub mod content;
mod dynamic_channel;
pub mod ext;
mod member_list;

use crate::weechat2::StyledString;
pub use color::*;
pub use dynamic_channel::*;
pub use member_list::*;

pub fn search_cached_striped_guild_name(
    cache: &InMemoryCache,
    target: &str,
) -> Option<CachedGuild> {
    crate::twilight_utils::search_striped_guild_name(
        cache,
        cache.iter().guilds().map(|g| *g.key()),
        target,
    )
}

pub fn search_striped_guild_name(
    cache: &InMemoryCache,
    guilds: impl IntoIterator<Item = GuildId>,
    target: &str,
) -> Option<CachedGuild> {
    for guild_id in guilds {
        if let Some(guild) = cache.guild(guild_id) {
            if utils::clean_name(guild.name()) == utils::clean_name(target) {
                return Some(guild.value().clone());
            }
        } else {
            tracing::warn!("{:?} not found in cache", guild_id);
        }
    }
    None
}

pub fn search_cached_stripped_guild_channel_name(
    cache: &InMemoryCache,
    guild_id: GuildId,
    target: &str,
) -> Option<GuildChannel> {
    let channels = cache
        .guild_channels(guild_id)
        .expect("guild_channels never fails");
    for channel_id in channels.iter() {
        if let Some(channel) = cache.guild_channel(*channel_id) {
            if !channel.is_text_channel(cache) {
                continue;
            }
            if utils::clean_name(channel.name()) == utils::clean_name(target) {
                return Some(channel.resource().clone());
            }
        } else {
            tracing::warn!("{:?} not found in cache", channel_id);
        }
    }
    None
}

pub fn current_user_nick(guild: &CachedGuild, cache: &InMemoryCache) -> StyledString {
    let current_user = cache
        .current_user()
        .expect("We have a connection, there must be a user");

    let member = cache.member(guild.id(), current_user.id);

    if let Some(member) = member {
        crate::utils::color::colorize_discord_member(cache, &member, false)
    } else {
        current_user.name.into()
    }
}
