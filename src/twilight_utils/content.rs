use crate::twilight_utils::{ext::CachedMemberExt, Mentionable};
use once_cell::sync::Lazy;
use regex::Regex;
use twilight_cache_inmemory::InMemoryCache;
use twilight_model::id::GuildId;

pub fn create_mentions(cache: &InMemoryCache, guild_id: Option<GuildId>, input: &str) -> String {
    let mut out = create_channels(cache, guild_id, input);
    out = create_users(cache, guild_id, &out);
    out = create_roles(cache, guild_id, &out);
    out = create_emojis(cache, guild_id, &out);

    out
}

pub fn create_channels(cache: &InMemoryCache, guild_id: Option<GuildId>, input: &str) -> String {
    static CHANNEL_MENTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"#([a-z_\-\d]+)").unwrap());

    let mut out = String::from(input);

    let matches = CHANNEL_MENTION.captures_iter(input).collect::<Vec<_>>();
    for channel_match in matches {
        let channel_name = channel_match
            .get(1)
            .expect("Regex contains exactly one group")
            .as_str();

        if let Some(guild_id) = guild_id {
            if let Some(channel_ids) = cache.guild_channels(guild_id) {
                for channel_id in channel_ids {
                    if let Some(channel) = cache.guild_channel(channel_id) {
                        if channel.name() == channel_name {
                            out = out.replace(
                                channel_match
                                    .get(0)
                                    .expect("group zero must exist")
                                    .as_str(),
                                &channel.id().mention(),
                            );
                        }
                    }
                }
            }
        }
    }

    out
}

pub fn create_users(cache: &InMemoryCache, guild_id: Option<GuildId>, input: &str) -> String {
    static USER_MENTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"@(.{0,32}?)#(\d{2,4})").unwrap());

    let mut out = String::from(input);

    let matches = USER_MENTION.captures_iter(input).collect::<Vec<_>>();
    for user_match in matches {
        let user_name = user_match
            .get(1)
            .expect("Regex contains exactly one group")
            .as_str();

        if let Some(guild_id) = guild_id {
            if let Some(members) = cache.members(guild_id) {
                for member in members {
                    if let Some(nick) = &member.nick {
                        if nick == user_name {
                            out = out.replace(
                                user_match.get(0).expect("group zero must exist").as_str(),
                                &member.user_id.mention(),
                            );
                        }
                    }

                    if member.user(cache).expect("FIX ME").name == user_name {
                        out = out.replace(
                            user_match.get(0).expect("group zero must exist").as_str(),
                            &member.user_id.mention(),
                        );
                    }
                }
            }
        }
    }

    out
}

pub fn create_roles(cache: &InMemoryCache, guild_id: Option<GuildId>, input: &str) -> String {
    static ROLE_MENTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"@([^\s]{1,32})").unwrap());

    let mut out = String::from(input);

    let matches = ROLE_MENTION.captures_iter(input).collect::<Vec<_>>();
    for role_match in matches {
        let role_name = role_match
            .get(1)
            .expect("Regex contains exactly one group")
            .as_str();

        if let Some(guild_id) = guild_id {
            if let Some(roles) = cache.roles(guild_id) {
                for role_id in roles {
                    if let Some(role) = cache.role(role_id) {
                        if role.name == role_name {
                            out = out.replace(
                                role_match.get(0).expect("group zero must exist").as_str(),
                                &role_id.mention(),
                            );
                        }
                    }
                }
            }
        }
    }

    out
}

pub fn create_emojis(cache: &InMemoryCache, guild_id: Option<GuildId>, input: &str) -> String {
    static EMOJI_MENTIONS: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\\?):(\w+):").unwrap());

    let mut out = String::from(input);

    let matches = EMOJI_MENTIONS.captures_iter(input).collect::<Vec<_>>();
    for emoji_match in matches {
        let emoji_prefix = emoji_match
            .get(1)
            .expect("Regex contains two groups")
            .as_str();

        if emoji_prefix == "\\" {
            continue;
        }

        let emoji_name = emoji_match
            .get(2)
            .expect("Regex contains two groups")
            .as_str();
        if let Some(guild_id) = guild_id {
            if let Some(emojis) = cache.emojis(guild_id) {
                for emoji_id in emojis {
                    if let Some(emoji) = cache.emoji(emoji_id) {
                        if emoji.name == emoji_name {
                            out = out.replace(
                                emoji_match.get(0).expect("group zero must exist").as_str(),
                                &emoji.mention(),
                            );
                        }
                    }
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {}
