use std::{
    cmp::Ordering,
    collections::VecDeque,
    time::{SystemTime, UNIX_EPOCH},
};

use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker, UserMarker},
    Id,
};

const MAX_TYPING_EVENTS: usize = 50;

#[derive(Debug, PartialEq, Eq)]
pub struct TypingEntry {
    pub channel_id: Id<ChannelMarker>,
    pub guild_id: Option<Id<GuildMarker>>,
    pub user: Id<UserMarker>,
    pub user_name: String,
    pub time: u64,
}

impl PartialOrd for TypingEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.time.partial_cmp(&other.time)
    }
}

impl Ord for TypingEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time)
    }
}

pub struct TypingTracker {
    entries: VecDeque<TypingEntry>,
}

impl TypingTracker {
    pub fn new() -> TypingTracker {
        TypingTracker {
            entries: VecDeque::new(),
        }
    }

    /// Remove any expired entries
    pub fn sweep(&mut self) {
        let now = SystemTime::now();
        let timestamp_now = now
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as u64;

        // If the entry is more than 10 seconds old, remove it
        // TODO: Use binary heap or other structure for better performance?
        self.entries.retain(|e| timestamp_now - e.time < 10);
    }

    /// Add a new entry
    pub fn add(&mut self, entry: TypingEntry) {
        self.entries.push_back(entry);

        self.sweep();
        if self.entries.len() > MAX_TYPING_EVENTS {
            self.entries.pop_front();
        }
    }

    /// Get the users currently typing
    pub fn typing(
        &self,
        guild_id: Option<Id<GuildMarker>>,
        channel_id: Id<ChannelMarker>,
    ) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.guild_id == guild_id && e.channel_id == channel_id)
            .map(|e| e.user_name.clone())
            .collect::<Vec<_>>()
    }
}
