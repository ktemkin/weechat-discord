use crate::{
    buffer::channel::Channel,
    config::{Config, GuildConfig},
    discord::discord_connection::ConnectionInner,
    instance::Instance,
    refcell::RefCell,
    twilight_utils::ext::ChannelExt,
};
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};
use twilight_cache_inmemory::model::CachedGuild as TwilightGuild;
use twilight_model::{
    channel::Channel as TwilightChannel,
    id::{
        marker::{ChannelMarker, GuildMarker},
        Id,
    },
};
use weechat::{
    buffer::{Buffer, BufferBuilder, BufferHandle},
    Weechat,
};

pub struct GuildBuffer(BufferHandle);

impl GuildBuffer {
    pub fn new(name: &str, id: Id<GuildMarker>, instance: Instance) -> anyhow::Result<Self> {
        let clean_guild_name = crate::utils::clean_name(name);
        let buffer_name = format!("discord.{}", clean_guild_name);

        let weechat = unsafe { Weechat::weechat() };

        let existing_buffer_handle = if let Some(buffer) =
            weechat.buffer_search(crate::PLUGIN_NAME, &buffer_name)
        {
            if !instance.borrow_guilds().contains_key(&id) {
                // TODO: Can we salvage an old buffer?  See corresponding comment in channel.rs
                tracing::trace!(guild.id=?id, buffer.name=%clean_guild_name, "Closing existing guild buffer not found in instance");
                buffer.close();
                None
            } else {
                tracing::trace!(guild.id=?id, buffer.name=%clean_guild_name, "Reusing guild buffer found in instance");
                Some(buffer.handle())
            }
        } else {
            None
        };

        tracing::debug!(guild.id=?id, buffer.name=%clean_guild_name, "Creating new guild buffer");
        let handle = match existing_buffer_handle {
            Some(buffer_handle) => buffer_handle,
            None => BufferBuilder::new(&buffer_name)
                .close_callback({
                    let name = name.to_owned();
                    move |_: &Weechat, _: &Buffer| {
                        tracing::trace!(buffer.id=%id, buffer.name=%name, "Guild buffer close");
                        Ok(())
                    }
                })
                .build()
                .map_err(|_| anyhow::anyhow!("Unable to create guild buffer"))?,
        };

        let buffer = handle
            .upgrade()
            .map_err(|_| anyhow::anyhow!("Unable to create guild buffer"))?;

        buffer.set_short_name(name);
        buffer.set_localvar("type", "server");
        buffer.set_localvar("server", &clean_guild_name);
        buffer.set_localvar("guild_id", &id.to_string());

        Ok(GuildBuffer(handle))
    }
}

pub struct GuildInner {
    conn: ConnectionInner,
    instance: Instance,
    guild: TwilightGuild,
    buffer: GuildBuffer,
    channels: HashSet<Id<ChannelMarker>>,
    closed: bool,
}

impl GuildInner {
    pub fn new(
        conn: ConnectionInner,
        instance: Instance,
        buffer: GuildBuffer,
        guild: TwilightGuild,
    ) -> Self {
        Self {
            conn,
            instance,
            buffer,
            guild,
            channels: HashSet::new(),
            closed: false,
        }
    }
}

impl Drop for GuildInner {
    fn drop(&mut self) {
        // This feels ugly, but without it, closing a buffer runs the close callback, which drops,
        // this struct, which in turn causes a segfault, as the buffer has already been explicitly
        // closed
        if self.closed {
            return;
        }
        if let Ok(buffer) = self.buffer.0.upgrade() {
            buffer.close();
        }
    }
}

#[derive(Clone)]
pub struct Guild {
    pub guild: TwilightGuild,
    pub id: Id<GuildMarker>,
    inner: Rc<RefCell<GuildInner>>,
    pub guild_config: GuildConfig,
    pub config: Config,
}

impl Guild {
    pub fn debug_counts(&self) -> (usize, usize) {
        (Rc::strong_count(&self.inner), Rc::weak_count(&self.inner))
    }

    fn new(
        guild: TwilightGuild,
        instance: Instance,
        conn: ConnectionInner,
        guild_config: GuildConfig,
        config: &Config,
    ) -> anyhow::Result<Guild> {
        let buffer = GuildBuffer::new(guild.name(), guild.id(), instance.clone())?;
        let inner = Rc::new(RefCell::new(GuildInner::new(
            conn,
            instance,
            buffer,
            guild.clone(),
        )));
        let guild = Guild {
            id: guild.id(),
            guild,
            inner,
            guild_config,
            config: config.clone(),
        };
        Ok(guild)
    }

    pub fn ensure_buffer_exists(&self) {
        let mut inner = self.inner.borrow_mut();

        if inner.closed || inner.buffer.0.upgrade().is_err() {
            if let Ok(buffer) =
                GuildBuffer::new(self.guild.name(), self.guild.id(), inner.instance.clone())
            {
                inner.closed = false;
                inner.buffer = buffer;
            }
        }
    }

    /// Tries to create a Guild and insert it into the instance, logging errors
    pub fn try_create(
        twilight_guild: &TwilightGuild,
        instance: &Instance,
        conn: &ConnectionInner,
        guild_config: GuildConfig,
        config: &Config,
    ) {
        let maybe_guild = instance
            .borrow_guilds_mut()
            .get(&twilight_guild.id())
            .cloned();
        match maybe_guild {
            Some(guild) => {
                guild.ensure_buffer_exists();
            },
            None => {
                match Self::new(
                    twilight_guild.clone(),
                    instance.clone(),
                    conn.clone(),
                    guild_config.clone(),
                    config,
                ) {
                    Ok(guild) => {
                        if guild_config.autoconnect() {
                            guild.try_join_channels();
                        }
                        instance.borrow_guilds_mut().insert(guild.id, guild);
                    },
                    Err(e) => {
                        tracing::error!(
                            guild.id=%twilight_guild.id(),
                            guild.name=%twilight_guild.name(),
                            "Unable to connect guild: {}", e
                        );
                    },
                }
            },
        }
    }

    pub fn try_join_channels(&self) {
        if let Err(e) = self.join_channels() {
            tracing::warn!("Unable to connect guild: {}", e);
            Weechat::print(&format!(
                "discord: Unable to connect to {}",
                self.inner.borrow().guild.name()
            ));
        };
    }

    fn join_channels(&self) -> anyhow::Result<()> {
        self.ensure_buffer_exists();
        let mut inner = self.inner.borrow_mut();

        let conn = inner.conn.clone();

        if self.config.join_all() {
            if let Some(guild_channels) = conn.cache.guild_channels(self.id) {
                for channel_id in guild_channels.iter() {
                    if let Some(cached_channel) = conn.cache.channel(*channel_id) {
                        if cached_channel.is_text_channel(&conn.cache) {
                            tracing::info!(
                                "Joining discord mode channel: #{}",
                                cached_channel.name()
                            );

                            self._join_channel(&cached_channel, &mut inner)?;
                        }
                    }
                }
            }
        } else {
            for channel_id in self.guild_config.autojoin_channels() {
                if let Some(cached_channel) = conn.cache.channel(channel_id) {
                    if cached_channel.is_text_channel(&conn.cache) {
                        tracing::info!("Joining autojoin channel: #{}", cached_channel.name());

                        self._join_channel(&cached_channel, &mut inner)?;
                    }
                }
            }

            for watched_channel_id in self.guild_config.watched_channels() {
                if let Some(channel) = conn.cache.channel(watched_channel_id) {
                    if let Some(read_state) = conn.cache.read_state(watched_channel_id) {
                        if Some(read_state.last_message_id) == channel.last_message_id {
                            continue;
                        };
                    } else {
                        tracing::warn!(
                            channel_id=?watched_channel_id,
                            "Unable to get read state for watched channel, skipping",
                        );
                        continue;
                    }

                    if channel.is_text_channel(&conn.cache) {
                        tracing::info!("Joining watched channel: #{}", channel.name());

                        self._join_channel(&channel, &mut inner)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn _join_channel(
        &self,
        channel: &TwilightChannel,
        inner: &mut GuildInner,
    ) -> anyhow::Result<Channel> {
        let channel_id = channel.id;
        let last_message_id = channel.last_message_id;
        // TODO: Reuse buffers like guilds
        let channel = crate::buffer::channel::Channel::guild(
            channel,
            &inner.guild,
            &inner.conn,
            &self.config,
            &inner.instance,
        )?;

        {
            let _old = inner
                .instance
                .borrow_channels_mut()
                .insert(channel_id, channel.clone());
        }
        inner.channels.insert(channel_id);

        if let Some(read_state) = inner.conn.cache.read_state(channel_id) {
            if last_message_id > Some(read_state.last_message_id) {
                channel.mark_unread(read_state.mention_count.map(|mc| mc > 0).unwrap_or(false));
            }
        }

        Ok(channel)
    }

    pub fn join_channel(&self, channel: &TwilightChannel) -> anyhow::Result<Channel> {
        self.ensure_buffer_exists();
        self._join_channel(channel, &mut self.inner.borrow_mut())
    }

    pub fn channels(&self) -> HashMap<Id<ChannelMarker>, Channel> {
        let inner = self.inner.borrow();
        let channels = inner.instance.borrow_channels();
        channels
            .iter()
            .filter(|(i, _)| inner.channels.contains(i))
            .map(|(i, c)| (*i, c.clone()))
            .collect()
    }

    pub fn set_closed(&self) {
        self.inner.borrow_mut().closed = true;
    }
}
