use crate::{
    buffer::ext::BufferExt,
    config::Config,
    discord::{discord_connection::DiscordConnection, typing_indicator::TypingTracker},
    instance::Instance,
    twilight_utils::ext::ChannelExt,
};
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker},
    Id,
};
use weechat::{buffer::Buffer, hooks::BarItem, Weechat};

pub struct BarItems {
    _typing: BarItem,
    _slowmode: BarItem,
    _readonly: BarItem,
}

impl BarItems {
    pub fn add_all(connection: DiscordConnection, instance: Instance, config: Config) -> BarItems {
        let _typing = BarItem::new("discord_typing", {
            move |_: &Weechat, buffer: &Buffer| {
                if let Some(channel_id) = buffer.channel_id() {
                    let guild_id = buffer.guild_id();

                    match config.typing_list_style() {
                        0 => terse_typing_list(
                            &instance,
                            channel_id,
                            guild_id,
                            config.typing_list_max() as usize,
                        ),
                        1 => expanded_typing_list(
                            &instance,
                            channel_id,
                            guild_id,
                            config.typing_list_max() as usize,
                        ),
                        _ => unreachable!(),
                    }
                } else {
                    "".into()
                }
            }
        })
        .expect("Unable to create typing bar item");

        let _slowmode = BarItem::new("discord_slowmode_cooldown", {
            let connection = connection.clone();
            move |_: &Weechat, buffer: &Buffer| {
                let connection = connection.borrow();
                let connection = match connection.as_ref() {
                    Some(conn) => conn,
                    None => return "".into(),
                };

                let channel_id = match buffer.channel_id() {
                    Some(channel_id) => channel_id,
                    None => return "".into(),
                };

                let channel = match connection.cache.channel(channel_id) {
                    Some(chan) => chan,
                    None => return "".into(),
                };

                match channel.rate_limit_per_user {
                    None => "".into(),
                    Some(rate_limit) => {
                        if rate_limit == 0 {
                            "".into()
                        } else {
                            humanize_duration(rate_limit)
                        }
                    },
                }
            }
        })
        .expect("Unable to create slowmode bar item");

        let _readonly = BarItem::new("discord_readonly", move |_: &Weechat, buffer: &Buffer| {
            let connection = connection.borrow();
            let connection = match connection.as_ref() {
                Some(conn) => conn,
                None => return "".into(),
            };

            let cache = &connection.cache;

            let channel_id = match buffer.channel_id() {
                Some(channel_id) => channel_id,
                None => return "".into(),
            };

            let channel = match cache.channel(channel_id) {
                Some(channel) => channel,
                None => return "".into(),
            };

            match channel.can_send(cache) {
                Some(false) => "🔒".into(),
                _ => "".into(),
            }
        })
        .expect("Unable to create readonly bar item");

        BarItems {
            _typing,
            _slowmode,
            _readonly,
        }
    }
}

fn terse_typing_list(
    instance: &Instance,
    channel_id: Id<ChannelMarker>,
    guild_id: Option<Id<GuildMarker>>,
    max_names: usize,
) -> String {
    let (head, has_more) = get_users_for_typing_list(
        &instance.borrow_typing_tracker_mut(),
        channel_id,
        guild_id,
        max_names,
    );

    let mut users = head.join(", ");
    if has_more {
        users += ", ...";
    }
    if users.is_empty() {
        "".into()
    } else {
        format!("typing: {}", users)
    }
}

fn expanded_typing_list(
    instance: &Instance,
    channel_id: Id<ChannelMarker>,
    guild_id: Option<Id<GuildMarker>>,
    max_names: usize,
) -> String {
    let (head, has_more) = get_users_for_typing_list(
        &instance.borrow_typing_tracker_mut(),
        channel_id,
        guild_id,
        max_names,
    );

    if head.is_empty() {
        "".into()
    } else if has_more {
        "Several people are typing...".into()
    } else if head.len() == 1 {
        format!("{} is typing", head[0])
    } else {
        let prefix = &head[..head.len() - 1];
        format!(
            "{} and {} are typing",
            prefix.join(", "),
            head[head.len() - 1]
        )
    }
}

fn get_users_for_typing_list(
    typing_tracker: &TypingTracker,
    channel_id: Id<ChannelMarker>,
    guild_id: Option<Id<GuildMarker>>,
    max_names: usize,
) -> (Vec<String>, bool) {
    let mut users = typing_tracker.typing(guild_id, channel_id);
    users.dedup();
    let (head, has_more) = if users.len() > max_names {
        (&users[..max_names], true)
    } else {
        (&users[..], false)
    };
    (head.to_vec(), has_more)
}

fn humanize_duration(duration: u64) -> String {
    match duration {
        0..=59 => format!("{}s", duration),
        60..=3599 => format!("{}m", duration / 60),
        3600..=86399 => format!("{}h", duration / 60 / 60),
        _ => format!("{}d", duration / 60 / 60 / 24),
    }
}

#[cfg(test)]
mod test {
    use crate::hooks::bar_items::humanize_duration;

    #[test]
    fn duration_fmt_second() {
        assert_eq!(humanize_duration(1), "1s");
        assert_eq!(humanize_duration(59), "59s");
        assert_eq!(humanize_duration(60), "1m");
    }

    #[test]
    fn duration_fmt_minute() {
        assert_eq!(humanize_duration(1 * 60), "1m");
        assert_eq!(humanize_duration(59 * 60), "59m");
        assert_eq!(humanize_duration(60 * 60), "1h");
    }

    #[test]
    fn duration_fmt_hour() {
        assert_eq!(humanize_duration(1 * 60 * 60), "1h");
        assert_eq!(humanize_duration(23 * 60 * 60), "23h");
        assert_eq!(humanize_duration(24 * 60 * 60), "1d");
    }

    #[test]
    fn duration_fmt_day() {
        assert_eq!(humanize_duration(1 * 60 * 60 * 24), "1d");
        assert_eq!(humanize_duration(59 * 60 * 60 * 24), "59d");
        assert_eq!(humanize_duration(60 * 60 * 60 * 24), "60d");
    }
}
