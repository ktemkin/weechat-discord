use twilight_cache_inmemory::{model::CachedMember, InMemoryCache};
use twilight_model::{
    channel::{permission_overwrite::PermissionOverwrite, ChannelType, GuildChannel},
    guild::Permissions,
    id::{MessageId, RoleId, UserId},
};

pub trait GuildChannelExt {
    fn permission_overwrites(&self, cache: &InMemoryCache) -> Vec<PermissionOverwrite>;
    fn topic(&self) -> Option<String>;
    fn members(&self, cache: &InMemoryCache) -> Result<Vec<CachedMember>, ()>;
    fn member_has_permission(
        &self,
        cache: &InMemoryCache,
        member: UserId,
        permissions: Permissions,
    ) -> Option<bool>;
    fn has_permission(&self, cache: &InMemoryCache, permissions: Permissions) -> Option<bool>;
    fn is_text_channel(&self, cache: &InMemoryCache) -> bool;
    fn last_message_id(&self) -> Option<MessageId>;
    fn rate_limit_per_user(&self) -> Option<u64>;
}

impl GuildChannelExt for GuildChannel {
    fn permission_overwrites(&self, cache: &InMemoryCache) -> Vec<PermissionOverwrite> {
        match self {
            GuildChannel::Category(c) => c.permission_overwrites.clone(),
            GuildChannel::Text(c) => c.permission_overwrites.clone(),
            GuildChannel::Voice(c) => c.permission_overwrites.clone(),
            GuildChannel::Stage(c) => c.permission_overwrites.clone(),
            GuildChannel::NewsThread(c) => c
                .parent_id
                .and_then(|parent_id| {
                    cache
                        .guild_channel(parent_id)
                        .map(|c| c.permission_overwrites(cache))
                })
                .unwrap_or_default(),
            GuildChannel::PrivateThread(c) => c.permission_overwrites.clone(),
            GuildChannel::PublicThread(c) => c
                .parent_id
                .and_then(|parent_id| {
                    cache
                        .guild_channel(parent_id)
                        .map(|c| c.permission_overwrites(cache))
                })
                .unwrap_or_default(),
        }
    }

    fn topic(&self) -> Option<String> {
        match self {
            GuildChannel::Text(c) => c.topic.clone(),
            GuildChannel::Category(_)
            | GuildChannel::Voice(_)
            | GuildChannel::Stage(_)
            | GuildChannel::NewsThread(_)
            | GuildChannel::PrivateThread(_)
            | GuildChannel::PublicThread(_) => None,
        }
    }

    fn members(&self, cache: &InMemoryCache) -> Result<Vec<CachedMember>, ()> {
        match self {
            GuildChannel::Category(_) | GuildChannel::Voice(_) | GuildChannel::Stage(_) => Err(()),
            GuildChannel::Text(channel) => {
                let guild_id = channel.guild_id.ok_or(())?;
                let members = cache
                    .iter()
                    .members()
                    .filter(|m| m.key().0 == guild_id)
                    .map(|m| m.value().clone());

                Ok(members
                    .into_iter()
                    .filter(|member| {
                        self.member_has_permission(
                            cache,
                            member.user_id(),
                            Permissions::READ_MESSAGE_HISTORY,
                        )
                        .unwrap_or(false)
                    })
                    .collect())
            },
            GuildChannel::NewsThread(_)
            | GuildChannel::PrivateThread(_)
            | GuildChannel::PublicThread(_) => Ok(vec![]),
            // GuildChannel::NewsThread(_) => todo!(),
            // GuildChannel::PrivateThread(_) => todo!(),
            // GuildChannel::PublicThread(_) => todo!(),
        }
    }

    fn member_has_permission(
        &self,
        cache: &InMemoryCache,
        member_id: UserId,
        permissions: Permissions,
    ) -> Option<bool> {
        let guild_id = self.guild_id().expect("guild channel must have a guild id");
        let member = cache.member(guild_id, member_id)?;

        let roles: Vec<_> = member
            .roles()
            .iter()
            .chain(Some(&RoleId(guild_id.0)))
            .flat_map(|&role_id| cache.role(role_id))
            .map(|role| (role.id, role.permissions))
            .collect();

        let everyone_role = cache.role(RoleId(guild_id.0)).map(|r| r.permissions)?;

        let calc = twilight_util::permission_calculator::PermissionCalculator::new(
            guild_id,
            member_id,
            everyone_role,
            &roles,
        );
        let perms = calc.in_channel(self.kind(), &self.permission_overwrites(cache));

        if perms.contains(permissions) {
            Some(true)
        } else {
            Some(false)
        }
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

        match self {
            GuildChannel::Category(c) => c.kind == ChannelType::GuildText,
            GuildChannel::Text(c) => c.kind == ChannelType::GuildText,
            GuildChannel::Voice(_) | GuildChannel::Stage(_) => false,
            // TODO: Verify if this comparison is correct
            GuildChannel::NewsThread(c) => c.kind == ChannelType::GuildNewsThread,
            GuildChannel::PrivateThread(c) => c.kind == ChannelType::GuildPrivateThread,
            GuildChannel::PublicThread(c) => c.kind == ChannelType::GuildPublicThread,
        }
    }

    fn last_message_id(&self) -> Option<MessageId> {
        match self {
            GuildChannel::Text(c) => c.last_message_id,
            GuildChannel::Category(_) | GuildChannel::Voice(_) | GuildChannel::Stage(_) => None,
            GuildChannel::NewsThread(c) => c.last_message_id,
            GuildChannel::PrivateThread(c) => c.last_message_id,
            GuildChannel::PublicThread(c) => c.last_message_id,
        }
    }

    fn rate_limit_per_user(&self) -> Option<u64> {
        match self {
            GuildChannel::Voice(_) | GuildChannel::Stage(_) | GuildChannel::Category(_) => None,
            GuildChannel::NewsThread(channel) => channel.rate_limit_per_user,
            GuildChannel::PrivateThread(channel) => channel.rate_limit_per_user,
            GuildChannel::PublicThread(channel) => channel.rate_limit_per_user,
            GuildChannel::Text(channel) => channel.rate_limit_per_user,
        }
    }
}
