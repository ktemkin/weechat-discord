use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::io;
use weechat::{
    buffer::{BufferBuilder, NotifyLevel},
    Weechat,
};

pub static TOKEN: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

pub struct Debug;

impl Debug {
    pub fn create_buffer() {
        if let Ok(buffer) = BufferBuilder::new("weecord.tracing").build() {
            if let Ok(buffer) = buffer.upgrade() {
                buffer.set_title("Tracing events for weecord");
                buffer.set_localvar("weecord_type", "debug_logs");
                buffer.disable_hotlist();
                // TODO: This currently overrides the notify level if the user has changed it.
                //       Perhaps an option needs to be added for running first time setup only once.
                buffer.set_notify(NotifyLevel::Never);
            }
        }
    }

    pub async fn write_to_buffer(msg: Vec<u8>) {
        let mut message = String::from_utf8(msg).unwrap();
        if let Some(token) = TOKEN.lock().as_ref() {
            message = message.replace(token, "<token redacted>");
        }
        let message = Weechat::execute_modifier("color_decode_ansi", "1", &message).unwrap();
        if let Some(buffer) =
            unsafe { Weechat::weechat() }.buffer_search("weecord", "weecord.tracing")
        {
            buffer.print(&message);
        }
    }
}

impl io::Write for Debug {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !crate::SHUTTING_DOWN.triggered() {
            #[cfg(not(feature = "unlimited-logging"))]
            let buf = &buf[0..(buf.len().min(4500))];
            Weechat::spawn_from_thread(Debug::write_to_buffer(buf.to_owned()));
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
