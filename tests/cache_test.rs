use twilight_cache_inmemory::InMemoryCache;
use twilight_model::{
    channel::{Channel, ChannelType, GuildChannel, TextChannel},
    gateway::payload::{ChannelCreate, GuildCreate, GuildEmojisUpdate, MemberAdd, RoleCreate},
    guild::{
        DefaultMessageNotificationLevel, Emoji, ExplicitContentFilter, Guild, Member, MfaLevel,
        NSFWLevel, Permissions, Role, SystemChannelFlags, VerificationLevel,
    },
    id::{EmojiId, GuildId, RoleId, UserId},
    user::User,
};

#[tokio::test]
async fn guild_emojis_updates() {
    let cache = InMemoryCache::new();
    let guild_id = GuildId(1);
    cache.update(&GuildCreate(fake_guild(guild_id)));

    assert!(cache.emojis(guild_id).unwrap().is_empty());
    let emojis = vec![Emoji {
        animated: false,
        available: false,
        id: EmojiId(1),
        managed: false,
        name: "".to_string(),
        require_colons: false,
        roles: vec![],
        user: None,
    }];
    cache.update(&GuildEmojisUpdate { emojis, guild_id });

    assert!(cache.emojis(guild_id).unwrap().contains(&EmojiId(1)));
}

#[tokio::test]
async fn guild_roles_updates() {
    let cache = InMemoryCache::new();
    let guild_id = GuildId(1);
    cache.update(&GuildCreate(fake_guild(guild_id)));

    assert!(cache.roles(guild_id).unwrap().is_empty());
    let role = Role {
        color: 0,
        hoist: false,
        id: RoleId(1),
        managed: false,
        mentionable: false,
        name: "foo".to_string(),
        permissions: Permissions::CREATE_INVITE,
        position: 0,
        tags: None,
    };
    cache.update(&RoleCreate { guild_id, role });

    assert!(dbg!(cache.roles(guild_id).unwrap()).contains(&RoleId(1)));
}

#[tokio::test]
async fn guild_members_updates() {
    let cache = InMemoryCache::new();
    let guild_id = GuildId(1);
    cache.update(&GuildCreate(fake_guild(guild_id)));

    assert!(cache.members(guild_id).unwrap().is_empty());
    let member = Member {
        deaf: false,
        guild_id,
        hoisted_role: None,
        joined_at: None,
        mute: false,
        nick: None,
        pending: false,
        premium_since: None,
        roles: vec![],
        user: User {
            accent_color: None,
            avatar: None,
            banner: None,
            bot: false,
            discriminator: "".to_string(),
            email: None,
            flags: None,
            id: UserId(1),
            locale: None,
            mfa_enabled: None,
            name: "".to_string(),
            premium_type: None,
            public_flags: None,
            system: None,
            verified: None,
        },
    };
    cache.update(&MemberAdd(member));

    assert_eq!(cache.members(guild_id).unwrap().len(), 1);
}

#[tokio::test]
async fn guild_channels_updates() {
    let cache = InMemoryCache::new();
    let guild_id = GuildId(1);
    cache.update(&GuildCreate(fake_guild(guild_id)));

    assert!(cache.guild_channels(GuildId(1)).unwrap().is_empty());
    let channel = GuildChannel::Text(TextChannel {
        guild_id: Some(guild_id),
        id: Default::default(),
        kind: ChannelType::GuildText,
        last_message_id: None,
        last_pin_timestamp: None,
        name: "".to_string(),
        nsfw: false,
        permission_overwrites: vec![],
        parent_id: None,
        position: 0,
        rate_limit_per_user: None,
        topic: None,
    });
    cache.update(&ChannelCreate(Channel::Guild(channel)));

    assert_eq!(cache.guild_channels(GuildId(1)).unwrap().len(), 1);
}

fn fake_guild(guild_id: GuildId) -> Guild {
    Guild {
        afk_channel_id: None,
        afk_timeout: 0,
        application_id: None,
        approximate_member_count: None,
        approximate_presence_count: None,
        banner: None,
        channels: Default::default(),
        default_message_notifications: DefaultMessageNotificationLevel::All,
        description: None,
        discovery_splash: None,
        emojis: Default::default(),
        explicit_content_filter: ExplicitContentFilter::None,
        features: vec![],
        icon: None,
        id: guild_id,
        joined_at: None,
        large: false,
        max_members: None,
        max_presences: None,
        max_video_channel_users: None,
        member_count: None,
        members: Default::default(),
        mfa_level: MfaLevel::None,
        name: "".to_string(),
        nsfw_level: NSFWLevel::Default,
        owner_id: Default::default(),
        owner: None,
        permissions: None,
        preferred_locale: "".to_string(),
        premium_subscription_count: None,
        premium_tier: Default::default(),
        presences: Default::default(),
        roles: Default::default(),
        rules_channel_id: None,
        splash: None,
        stage_instances: vec![],
        system_channel_flags: SystemChannelFlags::from_bits(0).unwrap(),
        system_channel_id: None,
        threads: vec![],
        unavailable: false,
        vanity_url_code: None,
        verification_level: VerificationLevel::None,
        voice_states: Default::default(),
        widget_channel_id: None,
        widget_enabled: None,
    }
}
