use crate::{
    twilight_utils::{ext::ChannelExt, Color},
    weechat2::{Style, StyledString},
};
use itertools::{Itertools, Position};
use parsing::MarkdownNode;
use std::{rc::Rc, sync::RwLock};
use twilight_cache_inmemory::InMemoryCache;
use twilight_model::id::{ChannelId, EmojiId, GuildId, RoleId, UserId};

struct FormattingState<'a> {
    cache: &'a InMemoryCache,
    guild_id: Option<GuildId>,
    show_unknown_ids: bool,
    show_formatting_chars: bool,
    unknown_members: &'a mut Vec<UserId>,
}

pub fn discord_to_weechat(
    msg: &str,
    cache: &InMemoryCache,
    guild_id: Option<GuildId>,
    show_formatting_chars: bool,
    show_unknown_ids: bool,
    unknown_members: &mut Vec<UserId>,
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
            let id = UserId::new(*id).expect("non zero");

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
                    format!("@{}", id.0).into()
                } else {
                    "@unknown-user".to_owned().into()
                }
            };
            out.append(mention);
            out
        },
        ChannelMention(id) => {
            let id = ChannelId::new(*id).expect("non zero");
            if let Some(channel) = state.cache.guild_channel(id) {
                out.push_str(&format!("#{}", channel.name()));
            } else if let Some(channel) = state.cache.private_channel(id) {
                out.push_str(&format!("#{}", channel.name()));
            } else if let Some(channel) = state.cache.group(id) {
                out.push_str(&format!("#{}", channel.name()));
            } else {
                out.push_str("#unknown-channel");
            }

            out
        },
        Emoji(_, id) => {
            if let Some(emoji) = state.cache.emoji(EmojiId::new(*id).expect("non zero")) {
                out.push_str(&format!(":{}:", emoji.name()));
            } else {
                tracing::trace!(emoji.id=?id, "Emoji not in cache");
                out.push_str(":unknown-emoji:");
            }
            out
        },
        RoleMention(id) => {
            if let Some(role) = state.cache.role(RoleId::new(*id).expect("non zero")) {
                let color = Style::color(&Color::new(role.color).as_8bit().to_string());
                out.push_style(color.clone());
                out.push_str(&format!("@{}", role.name));
                out.pop_style(color);
            } else {
                out.push_str("@unknown-role");
            }
            out
        },
    }
}

#[cfg(feature = "syntax_highlighting")]
mod syntax {
    use crate::{twilight_utils::Color, Weechat2};
    use once_cell::sync::Lazy;
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
                let ranges: Vec<(Style, &str)> = h.highlight(line, &PS);
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
            o.push_str(&format!("{}{}{}", color, str, resetcolor));
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
        channel::{Channel, ChannelType, GuildChannel, TextChannel},
        datetime::Timestamp,
        gateway::payload::incoming::{ChannelCreate, GuildEmojisUpdate, MemberAdd, RoleCreate},
        guild::{Emoji, Member, Permissions, Role},
        id::{ChannelId, EmojiId, GuildId, RoleId, UserId},
        user::User,
    };

    fn format(str: &str) -> String {
        format_with_cache(str, &InMemoryCache::new(), None)
    }

    fn format_with_cache(str: &str, cache: &InMemoryCache, guild_id: Option<GuildId>) -> String {
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
            id: RoleId::new(1).expect("non zero"),
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
            guild_id: GuildId::new(1).expect("non zero"),
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
        let guild_id = Some(GuildId::new(1).expect("non zero"));
        let channel = GuildChannel::Text(TextChannel {
            guild_id,
            id: ChannelId::new(1).expect("non zero"),
            kind: ChannelType::GuildText,
            last_message_id: None,
            last_pin_timestamp: None,
            name: "channel-one".to_string(),
            nsfw: false,
            permission_overwrites: vec![],
            parent_id: None,
            position: 0,
            rate_limit_per_user: None,
            topic: None,
        });
        cache.update(&ChannelCreate(Channel::Guild(channel)));

        assert_eq!(
            format_with_cache("hello <#1>!", &cache, guild_id),
            "hello #channel-one!"
        );
    }

    // TODO: Expand this, to test members, users, show_unkown, and the unknown_users aspects
    #[test]
    fn users() {
        let guild_id = GuildId::new(1).expect("non zero");

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
                id: UserId::new(1).expect("non zero"),
                locale: None,
                mfa_enabled: None,
                name: "random-user".to_string(),
                premium_type: None,
                public_flags: None,
                system: None,
                verified: None,
            },
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
                id: EmojiId::new(1).expect("non zero"),
                managed: false,
                name: "random-emoji".to_string(),
                require_colons: false,
                roles: vec![],
                user: None,
            },
            Emoji {
                animated: false,
                available: false,
                id: EmojiId::new(2).expect("non zero"),
                managed: false,
                name: "emoji-two".to_string(),
                require_colons: false,
                roles: vec![],
                user: None,
            },
        ];
        cache.update(&GuildEmojisUpdate {
            emojis,
            guild_id: GuildId::new(1).expect("non zero"),
        });

        assert_eq!(
            format_with_cache("hello <:random-emoji:1> <:emoji-two:2>", &cache, None,),
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
                id: EmojiId::new(i as u64 + 1).expect("non zero"),
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
            guild_id: GuildId::new(1).expect("non zero"),
        });
        let src = "<:one:1><:two:2><:one:1><:three:3><:four:4><:five:5><:one:1><:six:6><:one:1>";
        let target = ":one::two::one::three::four::five::one::six::one:";
        assert_eq!(format_with_cache(src, &cache, None), target);
    }
}
