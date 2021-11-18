use crate::{
    config::Config, discord::discord_connection::ConnectionInner, refcell::RefCell,
    twilight_utils::ext::ChannelExt, weecord_renderer::WeecordRenderer,
};
use std::rc::Rc;
use twilight_model::id::{ChannelId, GuildId};
use weechat::{
    buffer::{Buffer, BufferBuilder},
    Weechat,
};

pub struct PinsBuffer(WeecordRenderer);

impl PinsBuffer {
    pub fn new(
        name: &str,
        guild_id: Option<GuildId>,
        channel_id: ChannelId,
        conn: &ConnectionInner,
        config: &Config,
    ) -> anyhow::Result<Self> {
        let clean_buffer_name = crate::utils::clean_name(name);
        let buffer_name = format!("discord.pins.{}", clean_buffer_name);

        let weechat = unsafe { Weechat::weechat() };

        if let Some(buffer) = weechat.buffer_search(crate::PLUGIN_NAME, &buffer_name) {
            buffer.close();
        };

        let handle = BufferBuilder::new(&buffer_name)
            .close_callback({
                let name = format!("Pins for {}", name);
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

        buffer.set_short_name(&format!("Pins for #{}", name));
        if let Some(guild_id) = guild_id {
            buffer.set_localvar("guild_id", &guild_id.0.to_string());
        }
        buffer.set_localvar("channel_id", &channel_id.0.to_string());
        buffer.set_localvar("weecord_type", "pins");

        Ok(PinsBuffer(WeecordRenderer::new(
            conn,
            Rc::new(handle),
            config,
        )))
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

        let name = cache
            .guild_channel(self.channel_id)
            .map(|c| c.name().to_owned())
            .or_else(|| cache.private_channel(self.channel_id).map(|c| c.name()))
            .unwrap_or_else(|| "Unknown Channel".to_owned());

        let pins_buffer =
            PinsBuffer::new(&name, self.guild_id, self.channel_id, &conn, &self.config)?;
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
