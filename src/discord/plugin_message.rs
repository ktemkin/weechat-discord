use twilight_model::{
    channel::Message,
    gateway::payload::incoming::{
        ChannelUpdate, MemberChunk, MemberListUpdate, MessageDelete, MessageUpdate, ReactionAdd,
        ReactionRemove, TypingStart,
    },
    user::CurrentUser,
};

pub enum PluginMessage {
    Ready { user: CurrentUser },
    MessageCreate { message: Box<Message> },
    MessageDelete { event: MessageDelete },
    MessageUpdate { message: Box<MessageUpdate> },
    MemberChunk(MemberChunk),
    TypingStart(TypingStart),
    ChannelUpdate(ChannelUpdate),
    ReactionAdd(Box<ReactionAdd>),
    MemberListUpdate(Box<MemberListUpdate>),
    ReactionRemove(Box<ReactionRemove>),
}
