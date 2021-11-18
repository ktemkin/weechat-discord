use crate::{
    config::Config, discord::discord_connection::ConnectionInner, refcell::RefCell,
    twilight_utils::ext::ShallowUser, weecord_renderer::WeecordRenderer,
};
use std::rc::Rc;

use twilight_cache_inmemory::model::CachedGuild as TwilightGuild;
use twilight_model::{
    channel::{GuildChannel, PrivateChannel},
    id::{ChannelId, GuildId},
    user::User,
};
use weechat::{
    buffer::{Buffer, BufferBuilder},
    Weechat,
};

pub struct PinsBuffer(WeecordRenderer);

impl PinsBuffer {
    pub fn new_private(
        channel: PrivateChannel,
        conn: &ConnectionInner,
        config: &Config,
    ) -> anyhow::Result<Self> {
        Self::new(
            &Self::private_buffer_id(&channel.recipients),
            &channel
                .recipients
                .iter()
                .map(|r| r.name())
                .collect::<Vec<_>>()
                .join(", "),
            None,
            None,
            channel.id,
            conn,
            config,
        )
    }

    pub fn new_guild(
        channel: GuildChannel,
        guild: TwilightGuild,
        conn: &ConnectionInner,
        config: &Config,
    ) -> anyhow::Result<Self> {
        let clean_guild_name = crate::utils::clean_name(&guild.name);
        let clean_channel_name = crate::utils::clean_name(&channel.name());
        let buffer_name = format!("discord.{}.{}.pins", clean_guild_name, clean_channel_name);

        Self::new(
            &buffer_name,
            channel.name(),
            Some(clean_guild_name),
            Some(guild.id),
            channel.id(),
            conn,
            config,
        )
    }

    pub fn new(
        buffer_name: &str,
        channel_display_name: &str,
        clean_guild_name: Option<String>,
        guild_id: Option<GuildId>,
        channel_id: ChannelId,
        conn: &ConnectionInner,
        config: &Config,
    ) -> anyhow::Result<Self> {
        let weechat = unsafe { Weechat::weechat() };

        if let Some(buffer) = weechat.buffer_search(crate::PLUGIN_NAME, &buffer_name) {
            buffer.close();
        };

        let handle = BufferBuilder::new(&buffer_name)
            .close_callback({
                let name = format!("Pins for #{}", channel_display_name);
                move |_: &Weechat, _: &Buffer| {
                    tracing::trace!(guild.id=?guild_id, channel.id=?channel_id, buffer.name=%name, "Pins buffer close");
                    Ok(())
                }
            })
            .build()
            .map_err(|_| anyhow::anyhow!("Unable to create pins buffer"))?;

        let buffer = handle
            .upgrade()
            .map_err(|_| anyhow::anyhow!("Unable to create pins buffer"))?;

        buffer.set_short_name(&format!("Pins for #{}", channel_display_name));
        if let Some(guild_id) = guild_id {
            buffer.set_localvar("guild_id", &guild_id.0.to_string());
        }
        buffer.set_localvar("channel_id", &channel_id.0.to_string());
        buffer.set_localvar("weecord_type", "pins");
        if let Some(clean_guild_name) = clean_guild_name {
            buffer.set_localvar("type", "channel");
            buffer.set_localvar("server", &clean_guild_name);
        }

        Ok(PinsBuffer(WeecordRenderer::new(
            conn,
            Rc::new(handle),
            config,
        )))
    }

    fn private_buffer_id(recipients: &[User]) -> String {
        format!(
            "discord.dm.{}.pins",
            &recipients
                .iter()
                .map(|u| crate::utils::clean_name(&u.name))
                .collect::<Vec<_>>()
                .join(".")
        )
    }
}

pub struct PinsInner {
    conn: ConnectionInner,
    buffer: Option<PinsBuffer>,
    closed: bool,
}

impl Drop for PinsInner {
    fn drop(&mut self) {
        // This feels ugly, but without it, closing a buffer causes this struct to drop, which in turn
        // causes a segfault (for some reason)
        if self.closed {
            return;
        }
        if let Some(buffer) = self.buffer.as_ref() {
            if let Ok(buffer) = buffer.0.buffer_handle().upgrade() {
                buffer.close();
            }
        }
    }
}

impl PinsInner {
    pub fn new(conn: ConnectionInner) -> Self {
        Self {
            conn,
            buffer: None,
            closed: false,
        }
    }
}

#[derive(Clone)]
pub struct Pins {
    pub(crate) guild_id: Option<GuildId>,
    pub(crate) channel_id: ChannelId,
    inner: Rc<RefCell<PinsInner>>,
    config: Config,
}

impl Pins {
    pub fn debug_counts(&self) -> (usize, usize) {
        (Rc::strong_count(&self.inner), Rc::weak_count(&self.inner))
    }

    pub fn new(
        guild_id: Option<GuildId>,
        channel_id: ChannelId,
        conn: ConnectionInner,
        config: &Config,
    ) -> Self {
        let inner = Rc::new(RefCell::new(PinsInner::new(conn)));
        Pins {
            guild_id,
            channel_id,
            inner,
            config: config.clone(),
        }
    }

    pub async fn load(&self) -> anyhow::Result<()> {
        tracing::trace!(guild.id=?self.guild_id, channel.id=?self.channel_id, "Loading pins");
        let conn = self.inner.borrow().conn.clone();
        let cache = &conn.cache;
        let rt = &conn.rt;

        let pins_buffer = match self.guild_id.and_then(|g| cache.guild(g)) {
            Some(guild) => {
                if let Some(channel) = cache.guild_channel(self.channel_id) {
                    PinsBuffer::new_guild(channel, guild, &conn, &self.config)
                } else {
                    Err(anyhow::anyhow!("Unable to find guild channel"))
                }
            },
            None => {
                if let Some(channel) = cache.private_channel(self.channel_id) {
                    PinsBuffer::new_private(channel, &conn, &self.config)
                } else {
                    Err(anyhow::anyhow!("Unable to find guild channel"))
                }
            },
        }?;
        self.inner.borrow_mut().buffer.replace(pins_buffer);

        let pins: anyhow::Result<_> = rt
            .spawn({
                let channel_id = self.channel_id;
                let http = conn.http.clone();
                async move { Ok(http.pins(channel_id).exec().await?.models().await?) }
            })
            .await
            .expect("Task is never aborted");
        let pins = pins?;

        self.inner
            .borrow()
            .buffer
            .as_ref()
            .expect("guaranteed to exist")
            .0
            .add_bulk_msgs(pins.into_iter().rev());

        Ok(())
    }

    pub fn set_closed(&self) {
        self.inner.borrow_mut().closed = true;
    }
}
