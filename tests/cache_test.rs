use twilight_cache_inmemory::InMemoryCache;
use twilight_model::{
    channel::{Channel, ChannelType},
    datetime::Timestamp,
    gateway::payload::incoming::{
        ChannelCreate, GuildCreate, GuildEmojisUpdate, MemberAdd, RoleCreate,
    },
    guild::{
        DefaultMessageNotificationLevel, Emoji, ExplicitContentFilter, Guild, Member, MfaLevel,
        NSFWLevel, Permissions, Role, SystemChannelFlags, VerificationLevel,
    },
    id::{marker::GuildMarker, Id},
    user::User,
};

#[tokio::test]
async fn guild_emojis_updates() {
    let cache = InMemoryCache::new();
    let guild_id = Id::new(1);
    cache.update(&GuildCreate(fake_guild(guild_id)));

    assert!(
        cache
            .iter()
            .emojis()
            .filter(|e| e.guild_id() == guild_id)
            .count()
            == 0
    );
    let emojis = vec![Emoji {
        animated: false,
        available: false,
        id: Id::new(1),
        managed: false,
        name: "".to_string(),
        require_colons: false,
        roles: vec![],
        user: None,
    }];
    cache.update(&GuildEmojisUpdate { emojis, guild_id });

    assert!(cache
        .iter()
        .emojis()
        .filter(|e| e.guild_id() == guild_id)
        .map(|e| *e.key())
        .collect::<Vec<_>>()
        .contains(&Id::new(1)));
}

#[tokio::test]
async fn guild_roles_updates() {
    let cache = InMemoryCache::new();
    let guild_id = Id::new(1);
    cache.update(&GuildCreate(fake_guild(guild_id)));

    assert!(
        cache
            .iter()
            .roles()
            .filter(|r| r.guild_id() == guild_id)
            .count()
            == 0
    );
    let role = Role {
        color: 0,
        icon: None,
        unicode_emoji: None,
        hoist: false,
        id: Id::new(1),
        managed: false,
        mentionable: false,
        name: "foo".to_string(),
        permissions: Permissions::CREATE_INVITE,
        position: 0,
        tags: None,
    };
    cache.update(&RoleCreate { guild_id, role });

    assert!(cache
        .iter()
        .roles()
        .filter(|r| r.guild_id() == guild_id)
        .map(|r| r.key().to_owned())
        .collect::<Vec<_>>()
        .contains(&Id::new(1)));
}

#[tokio::test]
async fn guild_members_updates() {
    let cache = InMemoryCache::new();
    let guild_id = Id::new(1);
    cache.update(&GuildCreate(fake_guild(guild_id)));

    assert!(
        cache
            .iter()
            .members()
            .filter(|m| m.guild_id() == guild_id)
            .count()
            == 0
    );
    let member = Member {
        avatar: None,
        deaf: false,
        guild_id,
        joined_at: Timestamp::from_secs(1_632_072_645).expect("non zero"),
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
            discriminator: 0,
            email: None,
            flags: None,
            id: Id::new(1),
            locale: None,
            mfa_enabled: None,
            name: "".to_string(),
            premium_type: None,
            public_flags: None,
            system: None,
            verified: None,
        },
        communication_disabled_until: None,
    };
    cache.update(&MemberAdd(member));

    assert_eq!(
        cache
            .iter()
            .members()
            .filter(|m| m.guild_id() == guild_id)
            .count(),
        1
    );
}

#[tokio::test]
async fn guild_channels_updates() {
    let cache = InMemoryCache::new();
    let guild_id = Id::new(1);
    cache.update(&GuildCreate(fake_guild(guild_id)));

    assert!(cache.guild_channels(guild_id).unwrap().is_empty());
    let channel = Channel {
        guild_id: Some(guild_id),
        id: Id::new(1),
        kind: ChannelType::GuildText,
        last_message_id: None,
        last_pin_timestamp: None,
        name: Some("".to_string()),
        nsfw: Some(false),
        permission_overwrites: Some(vec![]),
        parent_id: None,
        position: Some(0),
        rate_limit_per_user: None,
        topic: None,
        application_id: None,
        bitrate: None,
        default_auto_archive_duration: None,
        icon: None,
        invitable: None,
        member: None,
        member_count: None,
        message_count: None,
        newly_created: None,
        owner_id: None,
        recipients: None,
        rtc_region: None,
        thread_metadata: None,
        user_limit: None,
        video_quality_mode: None,
    };
    cache.update(&ChannelCreate(channel));

    assert_eq!(cache.guild_channels(guild_id).unwrap().len(), 1);
}

fn fake_guild(guild_id: Id<GuildMarker>) -> Guild {
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
        owner_id: Id::new(1),
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
        stickers: vec![],
        system_channel_flags: SystemChannelFlags::from_bits(0).unwrap(),
        system_channel_id: None,
        threads: vec![],
        unavailable: false,
        vanity_url_code: None,
        verification_level: VerificationLevel::None,
        voice_states: Default::default(),
        widget_channel_id: None,
        widget_enabled: None,
        premium_progress_bar_enabled: false,
    }
}
