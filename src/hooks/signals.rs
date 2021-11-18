use crate::{
    buffer::{channel::Channel, ext::BufferExt},
    config::Config,
    discord::discord_connection::DiscordConnection,
    instance::Instance,
};
use once_cell::sync::Lazy;
use std::{
    borrow::{Borrow, Cow},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use twilight_model::id::{ChannelId, GuildId};
use weechat::{
    hooks::{SignalData, SignalHook},
    ReturnCode, Weechat,
};

pub struct Signals {
    _buffer_closing_hook: SignalHook,
    _buffer_switch_hook: SignalHook,
    _buffer_typing_hook: SignalHook,
}

impl Signals {
    pub fn hook_all(connection: DiscordConnection, instance: Instance, config: Config) -> Signals {
        let inner_connection = connection.clone();

        let _buffer_closing_hook = SignalHook::new("buffer_closing", {
            let instance = instance.clone();
            move |_: &Weechat, _: &str, data: Option<SignalData>| {
                if let Some(SignalData::Buffer(buffer)) = data {
                    if buffer.is_weecord_buffer() {
                        tracing::trace!(name = %buffer.full_name(), "Buffer close");
                        let guild_id = buffer.guild_id();
                        let channel_id = buffer.channel_id();

                        let buffer_type = buffer.weecord_buffer_type();
                        match buffer_type.as_ref().map(Cow::borrow) {
                            Some("guild") => {
                                if let Some(guild_id) = guild_id {
                                    if let Some(buf) =
                                        instance.borrow_guilds_mut().remove(&guild_id)
                                    {
                                        buf.set_closed();
                                    }
                                } else {
                                    tracing::warn!("guild type buffer has no guild id");
                                }
                            },
                            Some("channel") => {
                                if let Some(channel_id) = channel_id {
                                    if let Some(buf) =
                                        instance.borrow_channels_mut().remove(&channel_id)
                                    {
                                        buf.set_closed();
                                    }
                                } else {
                                    tracing::warn!("channel type buffer has no channel id");
                                }
                            },
                            Some("pins") => {
                                let channel_id = match channel_id {
                                    Some(channel_id) => channel_id,
                                    None => {
                                        tracing::warn!("pins type buffer has no channel id");
                                        return ReturnCode::Ok;
                                    },
                                };

                                if let Some(buf) =
                                    instance.borrow_pins_mut().remove(&(guild_id, channel_id))
                                {
                                    buf.set_closed();
                                }
                            },
                            _ => {},
                        };
                    }
                }
                ReturnCode::Ok
            }
        })
        .expect("Unable to hook buffer_closed signal");

        let _buffer_switch_hook = SignalHook::new("buffer_switch", {
            move |_: &Weechat, _: &str, data: Option<SignalData>| {
                if let Some(SignalData::Buffer(buffer)) = data {
                    let loaded = buffer.history_loaded();

                    let guild_id = buffer.guild_id();

                    let channel_id = match buffer.channel_id() {
                        Some(channel_id) => channel_id,
                        None => {
                            return ReturnCode::Ok;
                        },
                    };

                    if let Some(channel) = instance.search_buffer(guild_id, channel_id) {
                        if loaded {
                            if let Some(guild_id) = guild_id {
                                if let Some(member_list) =
                                    instance.borrow_member_lists().get(&guild_id)
                                {
                                    if let Some(conn) = inner_connection.borrow().as_ref() {
                                        if let Some(channel_memberlist) = member_list
                                            .get_list_for_channel(channel_id, &conn.cache)
                                        {
                                            channel.update_nicklist(channel_memberlist);
                                        }
                                    };
                                }
                            }
                            Weechat::spawn(async move {
                                Signals::ack(guild_id, channel_id, &channel).await;
                            })
                            .detach();
                        } else {
                            buffer.set_history_loaded();
                            let connection = inner_connection.clone();
                            Weechat::spawn(async move {
                                tracing::trace!(
                                    ?guild_id,
                                    ?channel_id,
                                    "Sending channel subscription"
                                );
                                if let Some(guild_id) = guild_id {
                                    connection
                                        .send_guild_subscription(guild_id, channel_id)
                                        .await;
                                }
                            })
                            .detach();
                            Weechat::spawn({
                                let channel = channel.clone();
                                async move {
                                    tracing::trace!(?guild_id, ?channel_id, "Loading history");
                                    if let Err(e) = channel.load_history().await {
                                        tracing::error!(
                                            ?guild_id,
                                            ?channel_id,
                                            "Error loading channel history: {}",
                                            e
                                        );
                                        Weechat::print(&format!(
                                            "discord: An error occurred loading history: {}",
                                            e
                                        ));
                                    }

                                    Signals::ack(guild_id, channel_id, &channel).await;
                                }
                            })
                            .detach();
                            if let Err(e) = channel.load_users(&instance) {
                                tracing::error!(
                                    ?guild_id,
                                    ?channel_id,
                                    "Error loading channel member list: {}",
                                    e
                                );
                            }
                        }
                    }
                }
                ReturnCode::Ok
            }
        })
        .expect("Unable to hook buffer_switch signal");

        let _buffer_typing_hook = SignalHook::new(
            "input_text_changed",
            move |_: &Weechat, _: &str, data: Option<SignalData>| {
                static LAST_TYPING_TIMESTAMP: Lazy<Arc<Mutex<u64>>> =
                    Lazy::new(|| Arc::new(Mutex::new(0)));

                if !config.send_typing() {
                    return ReturnCode::Ok;
                }

                if let Some(SignalData::Buffer(buffer)) = data {
                    if buffer.input().len() < 2 || buffer.input().starts_with('/') {
                        return ReturnCode::Ok;
                    }

                    // TODO: Wait for user to type for 3 seconds
                    let now = SystemTime::now();
                    let timestamp_now = now
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_secs() as u64;

                    if *LAST_TYPING_TIMESTAMP.lock().unwrap() + 9 < timestamp_now {
                        *LAST_TYPING_TIMESTAMP.lock().unwrap() = timestamp_now;

                        if let Some(channel_id) = buffer.channel_id() {
                            if let Some(conn) = connection.borrow().as_ref() {
                                let http = conn.http.clone();
                                conn.rt.spawn(async move {
                                    tracing::trace!(?channel_id, "Sending typing event");
                                    if let Err(e) =
                                        http.create_typing_trigger(channel_id).exec().await
                                    {
                                        tracing::error!("Sending typing start failed: {:#?}", e);
                                    };
                                });
                            }
                        }
                    }
                }
                ReturnCode::Ok
            },
        )
        .expect("Unable to hook input_text_changed signal");

        Signals {
            _buffer_closing_hook,
            _buffer_switch_hook,
            _buffer_typing_hook,
        }
    }

    async fn ack(guild_id: Option<GuildId>, channel_id: ChannelId, channel: &Channel) {
        tracing::trace!(?guild_id, ?channel_id, "Acking history");

        if let Err(e) = channel.ack().await {
            tracing::error!(
                ?guild_id,
                ?channel_id,
                "Error acking channel history: {}",
                e
            );
        }
    }
}
