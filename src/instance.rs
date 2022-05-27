use crate::{
    buffer::{channel::Channel, guild::Guild, pins::Pins},
    discord::typing_indicator::TypingTracker,
    twilight_utils::MemberList,
};
use parking_lot::{
    lock_api::{RwLockReadGuard, RwLockWriteGuard},
    RawRwLock, RwLock,
};
use std::{collections::HashMap, rc::Rc};
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker},
    Id,
};

#[derive(Clone)]
pub struct Instance {
    guilds: Rc<RwLock<HashMap<Id<GuildMarker>, Guild>>>,
    channels: Rc<RwLock<HashMap<Id<ChannelMarker>, Channel>>>,
    private_channels: Rc<RwLock<HashMap<Id<ChannelMarker>, Channel>>>,
    pins: Rc<RwLock<HashMap<(Option<Id<GuildMarker>>, Id<ChannelMarker>), Pins>>>,
    typing_tracker: Rc<RwLock<TypingTracker>>,
    member_lists: Rc<RwLock<HashMap<Id<GuildMarker>, MemberList>>>,
}

impl Instance {
    pub fn new() -> Self {
        Self {
            guilds: Rc::new(RwLock::new(HashMap::new())),
            channels: Rc::new(RwLock::new(HashMap::new())),
            private_channels: Rc::new(RwLock::new(HashMap::new())),
            pins: Rc::new(RwLock::new(HashMap::new())),
            typing_tracker: Rc::new(RwLock::new(TypingTracker::new())),
            member_lists: Rc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn borrow_guilds(&self) -> RwLockReadGuard<'_, RawRwLock, HashMap<Id<GuildMarker>, Guild>> {
        self.guilds.read()
    }

    #[allow(dead_code)]
    pub fn try_borrow_guilds_mut(
        &self,
    ) -> Option<RwLockWriteGuard<'_, RawRwLock, HashMap<Id<GuildMarker>, Guild>>> {
        self.guilds.try_write()
    }

    pub fn borrow_guilds_mut(
        &self,
    ) -> RwLockWriteGuard<'_, RawRwLock, HashMap<Id<GuildMarker>, Guild>> {
        self.guilds.write()
    }

    pub fn borrow_channels(
        &self,
    ) -> RwLockReadGuard<'_, RawRwLock, HashMap<Id<ChannelMarker>, Channel>> {
        self.channels.read()
    }

    pub fn borrow_channels_mut(
        &self,
    ) -> RwLockWriteGuard<'_, RawRwLock, HashMap<Id<ChannelMarker>, Channel>> {
        self.channels.write()
    }

    pub fn borrow_private_channels(
        &self,
    ) -> RwLockReadGuard<'_, RawRwLock, HashMap<Id<ChannelMarker>, Channel>> {
        self.private_channels.read()
    }

    pub fn borrow_private_channels_mut(
        &self,
    ) -> RwLockWriteGuard<'_, RawRwLock, HashMap<Id<ChannelMarker>, Channel>> {
        self.private_channels.write()
    }

    pub fn borrow_pins_mut(
        &self,
    ) -> RwLockWriteGuard<'_, RawRwLock, HashMap<(Option<Id<GuildMarker>>, Id<ChannelMarker>), Pins>>
    {
        self.pins.write()
    }

    pub fn borrow_typing_tracker_mut(&self) -> RwLockWriteGuard<'_, RawRwLock, TypingTracker> {
        self.typing_tracker.write()
    }

    pub fn search_buffer(
        &self,
        guild_id: Option<Id<GuildMarker>>,
        channel_id: Id<ChannelMarker>,
    ) -> Option<Channel> {
        if let Some(guild_id) = guild_id {
            if let Some(guild) = self.guilds.read().get(&guild_id) {
                return guild.channels().get(&channel_id).cloned();
            }
        } else {
            return self.private_channels.read().get(&channel_id).cloned();
        }

        None
    }

    pub fn borrow_member_lists(
        &self,
    ) -> RwLockReadGuard<'_, RawRwLock, HashMap<Id<GuildMarker>, MemberList>> {
        self.member_lists.read()
    }

    pub fn borrow_member_lists_mut(
        &self,
    ) -> RwLockWriteGuard<'_, RawRwLock, HashMap<Id<GuildMarker>, MemberList>> {
        self.member_lists.write()
    }
}
