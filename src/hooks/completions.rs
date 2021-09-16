use crate::{
    discord::discord_connection::DiscordConnection,
    twilight_utils::ext::{GuildChannelExt, UserExt},
    utils,
};
use std::borrow::Cow;
use weechat::{
    buffer::Buffer,
    hooks::{Completion, CompletionHook},
    Weechat,
};

pub struct Completions {
    _guild_completion_hook: CompletionHook,
    _channel_completion_hook: CompletionHook,
    _dm_completion_hook: CompletionHook,
}

impl Completions {
    pub fn hook_all(connection: DiscordConnection) -> Completions {
        let connection_clone = connection.clone();
        let _guild_completion_hook = CompletionHook::new(
            "discord_guild",
            "Completion for Discord servers",
            move |_: &Weechat, _: &Buffer, _: Cow<str>, completion: &Completion| {
                // `list` should not have any completion items
                if completion
                    .arguments()
                    .and_then(|a| a.splitn(3, ' ').nth(1).map(ToString::to_string))
                    .as_deref()
                    == Some("list")
                {
                    return Ok(());
                }

                if let Some(connection) = connection_clone.borrow().as_ref() {
                    let cache = connection.cache.clone();
                    let guilds = cache.guilds().expect("guilds never fails");
                    for guild_id in guilds {
                        if let Some(guild) = cache.guild(guild_id) {
                            completion.add(&utils::clean_name(&guild.name));
                        }
                    }
                }
                Ok(())
            },
        )
        .expect("Unable to hook discord guild completion");

        let connection_clone = connection.clone();
        let _channel_completion_hook = CompletionHook::new(
            "discord_channel",
            "Completion for Discord channels",
            move |_: &Weechat, _: &Buffer, _: Cow<str>, completion: &Completion| {
                // Get the previous argument which should be the guild name
                let guild_name = match completion
                    .arguments()
                    .and_then(|a| a.splitn(4, ' ').nth(2).map(ToString::to_string))
                {
                    Some(guild_name) => guild_name,
                    None => return Err(()),
                };
                let connection = connection_clone.borrow();
                let connection = match connection.as_ref() {
                    Some(connection) => connection,
                    None => return Err(()),
                };

                let cache = connection.cache.clone();

                match crate::twilight_utils::search_cached_striped_guild_name(&cache, &guild_name) {
                    Some(guild) => {
                        if let Some(channels) = cache.guild_channels(guild.id) {
                            for channel_id in channels {
                                match cache.guild_channel(channel_id) {
                                    Some(channel) => {
                                        if !channel.is_text_channel(&cache) {
                                            continue;
                                        }
                                        completion.add(&utils::clean_name(channel.name()));
                                    },
                                    None => {
                                        tracing::trace!(id = %channel_id, "Unable to find channel in cache");
                                    },
                                }
                            }
                        }
                    },
                    None => {
                        tracing::trace!(name = %guild_name, "Unable to find guild");
                    },
                }
                Ok(())
            },
        )
        .expect("Unable to hook discord channel completion");

        let connection_clone = connection;
        let _dm_completion_hook = CompletionHook::new(
            "discord_dm",
            "Completion for Discord private channels",
            move |_: &Weechat, _: &Buffer, _: Cow<str>, completion: &Completion| {
                if let Some(connection) = connection_clone.borrow().as_ref() {
                    for channel in &connection.cache.private_channels().expect("is always Some") {
                        completion.add(
                            &channel
                                .recipients
                                .iter()
                                .map(|u| crate::utils::clean_name_with_case(&u.tag()))
                                .collect::<Vec<_>>()
                                .join(","),
                        );
                    }
                }
                Ok(())
            },
        )
        .expect("Unable to hook discord guild completion");

        Completions {
            _guild_completion_hook,
            _channel_completion_hook,
            _dm_completion_hook,
        }
    }
}
