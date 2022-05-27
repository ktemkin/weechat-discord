use twilight_cache_inmemory::InMemoryCache;
use twilight_model::{
    channel::{permission_overwrite::PermissionOverwrite, Channel, ChannelType},
    gateway::payload::incoming::MemberListId,
    guild::Permissions,
    id::{
        marker::{RoleMarker, UserMarker},
        Id,
    },
};

use crate::twilight_utils::ext::UserExt;

pub trait ChannelExt {
    fn name(&self) -> String;
    fn can_send(&self, cache: &InMemoryCache) -> Option<bool>;
    fn has_permission(&self, cache: &InMemoryCache, permissions: Permissions) -> Option<bool>;
    fn is_text_channel(&self, cache: &InMemoryCache) -> bool;
    fn member_list_id(&self, cache: &InMemoryCache) -> MemberListId;
    fn member_has_permission(
        &self,
        cache: &InMemoryCache,
        member_id: Id<UserMarker>,
        permissions: Permissions,
    ) -> Option<bool>;
    fn permission_overwrites(&self, cache: &InMemoryCache) -> Vec<PermissionOverwrite>;
}

impl ChannelExt for Channel {
    fn name(&self) -> String {
        use twilight_model::channel::ChannelType::*;
        match self.kind {
            Group | Private => format!(
                "DM with {}",
                self.recipients
                    .as_ref()
                    .expect("channel to have receipients")
                    .iter()
                    .map(UserExt::tag)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            GuildText | GuildVoice | GuildCategory | GuildNews | GuildStore | GuildNewsThread
            | GuildPublicThread | GuildPrivateThread | GuildStageVoice | GuildDirectory
            | GuildForum => self.name.clone().expect("guild channel to have name"),
        }
    }

    fn can_send(&self, cache: &InMemoryCache) -> Option<bool> {
        self.has_permission(cache, Permissions::SEND_MESSAGES)
    }

    fn has_permission(&self, cache: &InMemoryCache, permissions: Permissions) -> Option<bool> {
        let current_user = cache.current_user()?;

        self.member_has_permission(cache, current_user.id, permissions)
    }

    fn is_text_channel(&self, cache: &InMemoryCache) -> bool {
        if !self
            .has_permission(
                cache,
                Permissions::READ_MESSAGE_HISTORY | Permissions::VIEW_CHANNEL,
            )
            .unwrap_or(false)
        {
            return false;
        }

        match self.kind {
            ChannelType::Private
            | ChannelType::Group
            | ChannelType::GuildNews
            | ChannelType::GuildNewsThread
            | ChannelType::GuildPublicThread
            | ChannelType::GuildPrivateThread
            | ChannelType::GuildText => true,
            ChannelType::GuildCategory
            | ChannelType::GuildStore
            | ChannelType::GuildStageVoice
            | ChannelType::GuildVoice
            | ChannelType::GuildForum
            | ChannelType::GuildDirectory => false,
        }
    }

    fn member_list_id(&self, cache: &InMemoryCache) -> MemberListId {
        match self.kind {
            ChannelType::Group | ChannelType::Private => MemberListId::Everyone,
            ChannelType::GuildText
            | ChannelType::GuildVoice
            | ChannelType::GuildCategory
            | ChannelType::GuildNews
            | ChannelType::GuildStore
            | ChannelType::GuildNewsThread
            | ChannelType::GuildPublicThread
            | ChannelType::GuildPrivateThread
            | ChannelType::GuildStageVoice
            | ChannelType::GuildDirectory
            | ChannelType::GuildForum => {
                let everyone_perms = cache
                    .role(Id::cast(
                        self.guild_id.expect("a guild channel must have a guild id"),
                    ))
                    .expect("Every guild has an @everyone role")
                    .permissions;
                MemberListId::from_overwrites(everyone_perms, &self.permission_overwrites(cache))
            },
        }
    }

    fn member_has_permission(
        &self,
        cache: &InMemoryCache,
        member_id: Id<UserMarker>,
        permissions: Permissions,
    ) -> Option<bool> {
        let guild_id = self.guild_id?;
        let member = cache.member(guild_id, member_id)?;

        let roles: Vec<_> = member
            .roles()
            .iter()
            .chain(Some(&guild_id.cast::<RoleMarker>()))
            .flat_map(|&role_id| cache.role(role_id))
            .map(|role| (role.id, role.permissions))
            .collect();

        let everyone_role = cache
            .role(guild_id.cast::<RoleMarker>())
            .map(|r| r.permissions)?;

        let calc = twilight_util::permission_calculator::PermissionCalculator::new(
            guild_id,
            member_id,
            everyone_role,
            &roles,
        );
        let perms = calc.in_channel(self.kind, &self.permission_overwrites(cache));

        if perms.contains(permissions) {
            Some(true)
        } else {
            Some(false)
        }
    }

    fn permission_overwrites(&self, cache: &InMemoryCache) -> Vec<PermissionOverwrite> {
        match self.kind {
            ChannelType::GuildText
            | ChannelType::Private
            | ChannelType::GuildVoice
            | ChannelType::Group
            | ChannelType::GuildCategory
            | ChannelType::GuildNews
            | ChannelType::GuildStore
            | ChannelType::GuildStageVoice
            | ChannelType::GuildDirectory
            | ChannelType::GuildForum => self.permission_overwrites.clone().unwrap_or_default(),
            ChannelType::GuildNewsThread
            | ChannelType::GuildPublicThread
            | ChannelType::GuildPrivateThread => self
                .parent_id
                .and_then(|parent_id| cache.channel(parent_id))
                .and_then(|c| c.permission_overwrites.clone())
                .unwrap_or_default(),
        }
    }
}
