use std::borrow::Cow;
use twilight_model::id::{ChannelId, GuildId};
use weechat::buffer::Buffer;

pub trait BufferExt {
    fn channel_id(&self) -> Option<ChannelId>;
    fn guild_id(&self) -> Option<GuildId>;

    fn history_loaded(&self) -> bool;
    fn set_history_loaded(&self);
    fn is_weecord_buffer(&self) -> bool;
    fn weecord_buffer_type(&self) -> Option<Cow<str>>;
}

impl BufferExt for Buffer<'_> {
    fn channel_id(&self) -> Option<ChannelId> {
        self.get_localvar("channel_id")
            .and_then(|ch| ch.parse().ok())
            .map(ChannelId)
    }

    fn guild_id(&self) -> Option<GuildId> {
        self.get_localvar("guild_id")
            .and_then(|ch| ch.parse().ok())
            .map(GuildId)
    }

    fn history_loaded(&self) -> bool {
        self.get_localvar("loaded_history").is_some()
    }

    fn set_history_loaded(&self) {
        self.set_localvar("loaded_history", "true");
    }

    fn is_weecord_buffer(&self) -> bool {
        self.plugin_name() == "weecord"
    }

    fn weecord_buffer_type(&self) -> Option<Cow<str>> {
        self.get_localvar("weecord_type")
    }
}
