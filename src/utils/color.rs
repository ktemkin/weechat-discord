use crate::{
    twilight_utils::ext::CachedMemberExt,
    weechat2::{Style, StyledString},
    Weechat2,
};
use twilight_cache_inmemory::{model::CachedMember, InMemoryCache};

pub fn colorize_string(text: &str, color: &str) -> StyledString {
    let mut builder = StyledString::new();
    if text.is_empty() || color.is_empty() {
        builder.push_str(text);
    } else {
        builder.push_styled_str(Style::color(color), text);
    }
    builder
}

pub fn colorize_discord_member(
    cache: &InMemoryCache,
    member: &CachedMember,
    at: bool,
) -> StyledString {
    let color = member.color(cache);
    let nick = member.display_name(cache);

    let nick_prefix = if at { "@" } else { "" };
    let nick = format!("{}{}", nick_prefix, nick);

    color
        .map(|color| colorize_string(&nick, &color.as_8bit().to_string()))
        .unwrap_or_else(|| {
            StyledString::from(format!(
                "{}{}",
                nick_prefix,
                member
                    .user(cache)
                    .map(|u| u.name)
                    .unwrap_or_else(|| String::from("<failed>"))
            ))
        })
}

pub fn colorize_weechat_nick(nick: &str, at: bool) -> StyledString {
    let color = Weechat2::info_get("nick_color_name", nick).unwrap_or_else(|| "reset".into());

    if at {
        colorize_string(&("@".to_owned() + nick), &color)
    } else {
        colorize_string(nick, &color)
    }
}
