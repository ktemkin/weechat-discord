use crate::{
    discord::discord_connection::ConnectionInner,
    twilight_utils::{ext::MemberExt, Color},
};
use std::{rc::Rc, sync::Arc};
use twilight_cache_inmemory::model::CachedMember;
use weechat::buffer::{BufferHandle, NickSettings};

pub struct Nicklist {
    conn: ConnectionInner,
    handle: Rc<BufferHandle>,
}

impl Nicklist {
    pub fn new(conn: &ConnectionInner, handle: Rc<BufferHandle>) -> Nicklist {
        Nicklist {
            conn: conn.clone(),
            handle,
        }
    }

    pub fn add_members(&self, members: &[Arc<CachedMember>]) {
        if let Ok(buffer) = self.handle.upgrade() {
            for member in members {
                let member_color = member
                    .color(&self.conn.cache)
                    .map(Color::as_8bit)
                    .filter(|&c| c != 0)
                    .map(|c| c.to_string());
                let mut nick_settings = NickSettings::new(&member.display_name());
                if let Some(ref member_color) = member_color {
                    nick_settings = nick_settings.set_color(member_color);
                }
                if let Some(role) = member.highest_role_info(&self.conn.cache) {
                    let role_color = Color::new(role.color).as_8bit().to_string();
                    if let Some(group) = buffer.search_nicklist_group(&role.name) {
                        if group.add_nick(nick_settings).is_err() {
                            tracing::error!(user.id=?member.user.id, group=%role.name, "Unable to add nick to nicklist");
                        }
                    } else if let Ok(group) =
                        buffer.add_nicklist_group(&role.name, &role_color, true, None)
                    {
                        if group.add_nick(nick_settings).is_err() {
                            tracing::error!(user.id=?member.user.id, group=%role.name, "Unable to add nick to nicklist");
                        }
                    } else if buffer.add_nick(nick_settings).is_err() {
                        tracing::error!(user.id=?member.user.id, "Unable to add nick to nicklist");
                    }
                } else if buffer.add_nick(nick_settings).is_err() {
                    tracing::error!(user.id=?member.user.id, "Unable to add nick to nicklist");
                }
            }
        }
    }
}
