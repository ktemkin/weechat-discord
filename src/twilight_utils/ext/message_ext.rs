use twilight_cache_inmemory::InMemoryCache;
use twilight_model::{
    channel::{message::Mention, Message},
    gateway::payload::incoming::MessageUpdate,
};

pub trait MessageExt {
    fn is_own(&self, cache: &InMemoryCache) -> bool;

    fn update(&mut self, update: MessageUpdate);
}

impl MessageExt for Message {
    fn is_own(&self, cache: &InMemoryCache) -> bool {
        let current_user = match cache.current_user() {
            Some(current_user) => current_user,
            None => return false,
        };

        self.author.id == current_user.id
    }

    fn update(&mut self, update: MessageUpdate) {
        self.id = update.id;
        self.channel_id = update.channel_id;
        self.edited_timestamp = update.edited_timestamp;
        for user in update.mentions.unwrap_or_default() {
            let mention = Mention {
                avatar: user.avatar.clone(),
                bot: user.bot,
                discriminator: user.discriminator,
                id: user.id,
                // TODO: Should this be populated somehow?
                member: None,
                name: user.name.clone(),
                public_flags: user.public_flags,
            };
            self.mentions.push(mention);
        }
        if let Some(attachments) = update.attachments {
            self.attachments = attachments;
        }
        if let Some(author) = update.author {
            self.author = author;
        }
        if let Some(content) = update.content {
            self.content = content;
        }
        if let Some(embeds) = update.embeds {
            self.embeds = embeds;
        }
        if let Some(kind) = update.kind {
            self.kind = kind;
        }
        if let Some(mention_everyone) = update.mention_everyone {
            self.mention_everyone = mention_everyone;
        }
        if let Some(mention_roles) = update.mention_roles {
            self.mention_roles = mention_roles;
        }
        if let Some(pinned) = update.pinned {
            self.pinned = pinned;
        }
        if let Some(timestamp) = update.timestamp {
            self.timestamp = timestamp;
        }
        if let Some(tts) = update.tts {
            self.tts = tts;
        }
    }
}
