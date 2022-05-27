use crate::{
    twilight_utils::{ext::ChannelExt, Color},
    weechat2::{Style, StyledString},
};
use chrono::{DateTime, Local, Offset};
use itertools::{Itertools, Position};
use parsing::MarkdownNode;
use std::{rc::Rc, sync::RwLock};
use time::{macros::format_description, Duration, OffsetDateTime};
use twilight_cache_inmemory::InMemoryCache;
use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};

struct FormattingState<'a> {
    cache: &'a InMemoryCache,
    guild_id: Option<Id<GuildMarker>>,
    show_unknown_ids: bool,
    show_formatting_chars: bool,
    unknown_members: &'a mut Vec<Id<UserMarker>>,
}

pub fn discord_to_weechat(
    msg: &str,
    cache: &InMemoryCache,
    guild_id: Option<Id<GuildMarker>>,
    show_formatting_chars: bool,
    show_unknown_ids: bool,
    unknown_members: &mut Vec<Id<UserMarker>>,
) -> StyledString {
    let mut state = FormattingState {
        cache,
        guild_id,
        show_unknown_ids,
        show_formatting_chars,
        unknown_members,
    };
    let ast = parsing::parse_markdown(msg);

    collect_children(&ast.0, &mut state)
}

fn collect_children(
    styles: &[Rc<RwLock<MarkdownNode>>],
    state: &mut FormattingState,
) -> StyledString {
    styles
        .iter()
        .map(|s| discord_to_weechat_reducer(&*s.read().unwrap(), state))
        .fold(StyledString::new(), |mut acc, x| {
            acc.append(x);
            acc
        })
}

trait Magic {
    fn if_do<'a, F: Fn(&'a mut Self) -> &mut Self>(
        &'a mut self,
        condition: bool,
        f: F,
    ) -> &'a mut Self;
}

impl Magic for StyledString {
    fn if_do<'a, F: Fn(&'a mut Self) -> &mut Self>(
        &'a mut self,
        condition: bool,
        f: F,
    ) -> &'a mut Self {
        if condition {
            f(self)
        } else {
            self
        }
    }
}

// TODO: if the whole line is wrapped in *, render as CTCP ACTION rather than
//       as fully italicized message.
#[allow(clippy::too_many_lines)]
fn discord_to_weechat_reducer(node: &MarkdownNode, state: &mut FormattingState) -> StyledString {
    use MarkdownNode::*;
    let show_fmt = state.show_formatting_chars;
    let mut out = StyledString::new();

    match node {
        Bold(children) => {
            out.push_style(Style::Bold)
                .if_do(show_fmt, |s| s.push_str("**"))
                .absorb(collect_children(children, state))
                .if_do(show_fmt, |s| s.push_str("**"))
                .pop_style(Style::Bold);
            out
        },
        Italic(children) => {
            out.push_style(Style::Italic)
                .if_do(show_fmt, |s| s.push_str("_"))
                .absorb(collect_children(children, state))
                .if_do(show_fmt, |s| s.push_str("_"))
                .pop_style(Style::Italic);
            out
        },
        Underline(children) => {
            out.push_style(Style::Underline)
                .if_do(show_fmt, |s| s.push_str("__"))
                .absorb(collect_children(children, state))
                .if_do(show_fmt, |s| s.push_str("__"))
                .pop_style(Style::Underline);
            out
        },
        Strikethrough(children) => {
            out.push_style(Style::Color("red".into()))
                .if_do(show_fmt, |s| s.push_str("~~"))
                .absorb(collect_children(children, state))
                .if_do(show_fmt, |s| s.push_str("~~"))
                .pop_style(Style::Color("red".into()));
            out
        },
        Spoiler(children) => {
            out.push_style(Style::Italic)
                .push_str("||")
                .absorb(collect_children(children, state))
                .push_str("||")
                .pop_style(Style::Italic);
            out
        },
        Text(string) => {
            out.push_str(string);
            out
        },
        InlineCode(string) => {
            out.push_style(Style::color("8"))
                .push_style(Style::Bold)
                .if_do(show_fmt, |s| s.push_str("`"))
                .push_str(string)
                .if_do(show_fmt, |s| s.push_str("`"))
                .pop_style(Style::Bold)
                .pop_style(Style::color("8"));

            out
        },
        Code(language, text) => {
            #[cfg(feature = "syntax_highlighting")]
            let text = syntax::format_code(text, language);

            #[allow(clippy::needless_borrow)]
            out.push_style(Style::Reset)
                .if_do(show_fmt, |s| s.push_str("```").push_str(language))
                .push_str("\n")
                .push_style(Style::color("8"))
                .push_style(Style::Bold)
                .push_str(&text)
                .pop_style(Style::Bold)
                .pop_style(Style::color("8"))
                .if_do(show_fmt, |s| s.push_str("\n```"))
                .pop_style(Style::Reset);
            out
        },
        BlockQuote(children) => {
            out.append(format_block_quote(
                collect_children(children, state).lines().into_iter(),
            ));
            out
        },
        SingleBlockQuote(children) => {
            out.append(format_block_quote(
                collect_children(children, state)
                    .lines()
                    .into_iter()
                    .map(strip_leading_bracket),
            ));
            out
        },
        UserMention(id) => {
            let id = Id::new(*id);

            let replacement = if let Some(guild_id) = state.guild_id {
                state.cache.member(guild_id, id).map(|member| {
                    crate::utils::color::colorize_discord_member(state.cache, &member, true)
                })
            } else {
                state
                    .cache
                    .user(id)
                    .map(|user| crate::utils::color::colorize_weechat_nick(&user.name, true))
            };

            let mention = if let Some(replacement) = replacement {
                replacement
            } else {
                state.unknown_members.push(id);

                if state.show_unknown_ids {
                    format!("@{}", id).into()
                } else {
                    "@unknown-user".to_owned().into()
                }
            };
            out.append(mention);
            out
        },
        ChannelMention(id) => {
            let id = Id::new(*id);
            if let Some(channel) = state.cache.channel(id) {
                out.push_str(&format!("#{}", channel.name()));
            // } else if let Some(channel) = state.cache.channel(id) {
            //     out.push_str(&format!("#{}", channel.name()));
            // } else if let Some(channel) = state.cache.group(id) {
            //     out.push_str(&format!("#{}", channel.name()));
            } else {
                out.push_str("#unknown-channel");
            }

            out
        },
        Emoji(_, id) => {
            if let Some(emoji) = state.cache.emoji(Id::new(*id)) {
                out.push_str(&format!(":{}:", emoji.name()));
            } else {
                tracing::trace!(emoji.id=?id, "Emoji not in cache");
                out.push_str(":unknown-emoji:");
            }
            out
        },
        RoleMention(id) => {
            if let Some(role) = state.cache.role(Id::new(*id)) {
                let color = Style::color(&Color::new(role.color).as_8bit().to_string());
                out.push_style(color.clone());
                out.push_str(&format!("@{}", role.name));
                out.pop_style(color);
            } else {
                out.push_str("@unknown-role");
            }
            out
        },
        Timestamp(time, style) => {
            out.if_do(show_fmt, |s| s.push_str("<"))
                .push_str(&fmt_timestamp(*time, style.unwrap_or('f')))
                .if_do(show_fmt, |s| s.push_str(">"));
            out
        },
    }
}

fn fmt_timestamp(timestamp: i64, style: char) -> String {
    // Temporary solution until `time` supports local offset in multirhreaded environments
    let local_timestamp: DateTime<Local> = chrono::Local::now();

    let offset_time = time::OffsetDateTime::from_unix_timestamp(
        timestamp + local_timestamp.offset().fix().local_minus_utc() as i64,
    )
    .unwrap();
    match style {
        't' => offset_time.format(format_description!("[hour]:[minute]")),
        'T' => offset_time.format(format_description!("[hour]:[minute]:[second]")),
        'd' => offset_time.format(format_description!("[day]/[month]/[year]")),
        'D' => offset_time.format(format_description!("[day] [month repr:long] [year]")),
        'f' => offset_time.format(format_description!(
            "[day] [month repr:long] [year] [hour]:[minute]"
        )),
        'F' => offset_time.format(format_description!(
            "[weekday repr:long], [day] [month repr:long] [year] [hour]:[minute]"
        )),
        'R' => Ok(humanize_relative_date(offset_time)),
        _ => Ok("<invalid timestamp>".to_owned()),
    }
    .unwrap()
}

fn humanize_relative_date(date: OffsetDateTime) -> String {
    let local_timestamp: DateTime<Local> = chrono::Local::now();

    let now = OffsetDateTime::from_unix_timestamp(
        local_timestamp.timestamp() + local_timestamp.offset().fix().local_minus_utc() as i64,
    )
    .unwrap();

    humanize_duration(now - date)
}

// Based on discord relative date formatting
fn humanize_duration(duration: Duration) -> String {
    match duration.whole_seconds() {
        n if n.abs() < 2 => {
            if n >= 0 {
                "just now".to_owned()
            } else {
                "now".to_owned()
            }
        },
        n if n.abs() < 60 => {
            if n >= 0 {
                format!("{} seconds ago", n)
            } else {
                format!("in {} seconds", -n)
            }
        },
        n if n.abs() < 120 => {
            if n >= 0 {
                "about a minute ago".to_owned()
            } else {
                "in about a minute".to_owned()
            }
        },
        n if n.abs() < 3600 => {
            if n >= 0 {
                format!("{} minutes ago", (n as f64 / 60.).floor())
            } else {
                format!("in {} minutes", (-n as f64 / 60.).floor())
            }
        },

        n if n.abs() < 7200 => {
            if n >= 0 {
                "about an hour ago".to_owned()
            } else {
                "in about an hour".to_owned()
            }
        },
        n if n.abs() < 86400 => {
            if n >= 0 {
                format!("{} hours ago", (n as f64 / 3600.).floor())
            } else {
                format!("in {} hours", (-n as f64 / 3600.).floor())
            }
        },
        n if n.abs() < 172800 => {
            if n >= 0 {
                "1 day ago".to_owned()
            } else {
                "in 1 day".to_owned()
            }
        },
        n if n.abs() < 2505600 => {
            if n >= 0 {
                format!("{} days ago", (n as f64 / 86400.).floor())
            } else {
                format!("in {} days", (-n as f64 / 86400.).floor())
            }
        },
        n if n.abs() < 5184000 => {
            if n >= 0 {
                "about a month ago".to_owned()
            } else {
                "in about a month".to_owned()
            }
        },
        n if n.abs() / 2505600 < 12 => {
            if n >= 0 {
                format!("{} months ago", (n as f64 / 2505600.).floor())
            } else {
                format!("in {} months", (-n as f64 / 2505600.).floor())
            }
        },
        _ => {
            let years = (duration.whole_days() as f64 / 365.).ceil() as i64;
            if years.abs() < 2 {
                if years >= 0 {
                    "a year ago".to_owned()
                } else {
                    "in a year".to_owned()
                }
            } else if years >= 0 {
                format!("{} years ago", years)
            } else {
                format!("in {} years", years)
            }
        },
    }
}

#[cfg(feature = "syntax_highlighting")]
mod syntax {
    use crate::{twilight_utils::Color, Weechat2};
    use once_cell::sync::Lazy;
    use std::fmt::Write;
    use syntect::{
        easy::HighlightLines,
        highlighting::{Style, ThemeSet},
        parsing::SyntaxSet,
        util::LinesWithEndings,
    };

    pub fn format_code(src: &str, language: &str) -> String {
        static PS: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
        static TS: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

        if let Some(syntax) = PS.find_syntax_by_token(language) {
            let mut h = HighlightLines::new(syntax, &TS.themes["Solarized (dark)"]);
            let mut out = String::new();
            for line in LinesWithEndings::from(src) {
                let ranges: Vec<(Style, &str)> =
                    h.highlight_line(line, &PS).expect("default theme to work");
                out.push_str(&syntect_as_weechat_escaped(&ranges[..]));
            }
            out
        } else {
            tracing::debug!("unable to find syntax for language: {}", language);
            src.to_owned()
        }
    }

    fn syntect_as_weechat_escaped(v: &[(Style, &str)]) -> String {
        let mut o = String::new();
        let resetcolor = Weechat2::color("resetcolor");
        for (style, str) in v {
            let fg = style.foreground;
            let fg = Color::from_rgb(fg.r, fg.g, fg.b);
            let colorstr = format!("{}", fg.as_8bit());
            let color = Weechat2::color(&colorstr);
            write!(o, "{}{}{}", color, str, resetcolor).expect("writing to a string to succeed");
        }
        o
    }
}

fn strip_leading_bracket(line: StyledString) -> StyledString {
    let start = line.find("> ").map_or(0, |x| x + 2);
    line.slice(start..)
}

pub fn fold_lines<S: Into<StyledString>>(
    lines: impl Iterator<Item = S>,
    sep: &str,
) -> StyledString {
    let mut out = StyledString::new();
    for line in lines.with_position() {
        let newlines = matches!(line, Position::First(_) | Position::Middle(_));
        out.push_str(sep);
        out.absorb(line.into_inner().into());
        if newlines {
            out.push_str("\n");
        }
    }
    out
}

fn format_block_quote(lines: impl Iterator<Item = StyledString>) -> StyledString {
    fold_lines(lines, "▎")
}

#[cfg(test)]
mod tests {
    use super::discord_to_weechat;
    use twilight_cache_inmemory::InMemoryCache;
    use twilight_model::{
        channel::{Channel, ChannelType},
        datetime::Timestamp,
        gateway::payload::incoming::{ChannelCreate, GuildEmojisUpdate, MemberAdd, RoleCreate},
        guild::{Emoji, Member, Permissions, Role},
        id::{marker::GuildMarker, Id},
        user::User,
    };

    fn format(str: &str) -> String {
        format_with_cache(str, &InMemoryCache::new(), None)
    }

    fn format_with_cache(
        str: &str,
        cache: &InMemoryCache,
        guild_id: Option<Id<GuildMarker>>,
    ) -> String {
        discord_to_weechat(str, cache, guild_id, true, false, &mut Vec::new()).build()
    }

    #[test]
    fn block() {
        assert_eq!(
            format(">>> **foo\n bar**"),
            "▎bold**foo-bold\n▎bold bar**-bold"
        );
    }

    #[test]
    fn color_stack() {
        assert_eq!(
            format("||foo ~~strikethrough~~ baz `code` spam||"),
            "italic||foo red~~strikethrough~~resetitalic baz 8bold`code`-boldresetitalic \
             spam||-italic"
        );
    }

    #[test]
    fn smoke_test() {
        assert_eq!(
            format("**_Hi___ there__**"),
            "bold**italic_Hi___-italic there__**-bold"
        );
        assert_eq!(format("A _b*c_d*e_"), "A _bitalic_c_d_-italice_");
        assert_eq!(
            format("__f_*o*_o__"),
            "underline__f_italic_o_-italic_o__-underline"
        );
    }

    #[test]
    fn roles() {
        let cache = InMemoryCache::new();
        let role = Role {
            color: 0,
            hoist: false,
            id: Id::new(1),
            icon: None,
            unicode_emoji: None,
            managed: false,
            mentionable: false,
            name: "foo".to_string(),
            permissions: Permissions::CREATE_INVITE,
            position: 0,
            tags: None,
        };
        cache.update(&RoleCreate {
            guild_id: Id::new(1),
            role,
        });

        assert_eq!(
            format_with_cache("hello <@&1>!", &cache, None),
            "hello 16@fooreset!"
        );
    }

    #[test]
    fn channels() {
        let cache = InMemoryCache::new();
        let guild_id = Some(Id::new(1));
        let channel = Channel {
            guild_id,
            id: Id::new(1),
            kind: ChannelType::GuildText,
            last_message_id: None,
            last_pin_timestamp: None,
            name: Some("channel-one".to_string()),
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

        assert_eq!(
            format_with_cache("hello <#1>!", &cache, guild_id),
            "hello #channel-one!"
        );
    }

    // TODO: Expand this, to test members, users, show_unkown, and the unknown_users aspects
    #[test]
    fn users() {
        let guild_id = Id::new(1);

        let cache = InMemoryCache::new();
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
                discriminator: 1234,
                email: None,
                flags: None,
                id: Id::new(1),
                locale: None,
                mfa_enabled: None,
                name: "random-user".to_string(),
                premium_type: None,
                public_flags: None,
                system: None,
                verified: None,
            },
            communication_disabled_until: None,
        };
        cache.update(&MemberAdd(member));

        assert_eq!(
            format_with_cache("hello <@1>!", &cache, Some(guild_id)),
            "hello @random-user!"
        );
        assert_eq!(
            format_with_cache("hello <@!1>!", &cache, Some(guild_id)),
            "hello @random-user!"
        );
    }

    #[test]
    fn emojis() {
        let cache = InMemoryCache::new();
        let emojis = vec![
            Emoji {
                animated: false,
                available: false,
                id: Id::new(1),
                managed: false,
                name: "random-emoji".to_string(),
                require_colons: false,
                roles: vec![],
                user: None,
            },
            Emoji {
                animated: false,
                available: false,
                id: Id::new(2),
                managed: false,
                name: "emoji-two".to_string(),
                require_colons: false,
                roles: vec![],
                user: None,
            },
        ];
        cache.update(&GuildEmojisUpdate {
            emojis,
            guild_id: Id::new(1),
        });

        assert_eq!(
            format_with_cache("hello <:random-emoji:1> <:emoji-two:2>", &cache, None),
            "hello :random-emoji: :emoji-two:"
        );
    }

    #[test]
    fn evil_pony() {
        let cache = InMemoryCache::new();
        let mut emojis = Vec::new();
        for (i, n) in ["one", "two", "three", "four", "five", "six"]
            .iter()
            .enumerate()
        {
            let emoji = Emoji {
                animated: false,
                available: false,
                id: Id::new(i as u64 + 1),
                managed: false,
                name: n.to_string(),
                require_colons: false,
                roles: vec![],
                user: None,
            };
            emojis.push(emoji);
        }
        cache.update(&GuildEmojisUpdate {
            emojis,
            guild_id: Id::new(1),
        });
        let src = "<:one:1><:two:2><:one:1><:three:3><:four:4><:five:5><:one:1><:six:6><:one:1>";
        let target = ":one::two::one::three::four::five::one::six::one:";
        assert_eq!(format_with_cache(src, &cache, None), target);
    }
}
