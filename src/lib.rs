#![warn(
    clippy::all,
    clippy::str_to_string,
    clippy::semicolon_if_nothing_returned,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms
)]
#![allow(
    elided_lifetimes_in_paths,
    clippy::module_name_repetitions,
    clippy::non_ascii_literal,
    clippy::single_match_else,
    clippy::enum_glob_use
)]
#![deny(clippy::await_holding_refcell_ref, clippy::await_holding_lock)]
use crate::{discord::discord_connection::DiscordConnection, instance::Instance, utils::Flag};
pub use refcell::RefCell;
use std::{error::Error, result::Result as StdResult};
use tokio::sync::mpsc::channel;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use weechat::{plugin, Args, Plugin, Weechat};
pub use weechat2::Weechat2;

mod buffer;
mod config;
mod discord;
mod hooks;
mod instance;
mod nicklist;
mod refcell;
mod twilight_utils;
mod utils;
mod weechat2;
mod weecord_renderer;

pub const PLUGIN_NAME: &str = "weecord";
pub static SHUTTING_DOWN: Flag = Flag::new();

pub struct Weecord {
    discord_connection: DiscordConnection,
    config: config::Config,
    instance: Instance,
    hooks: Option<hooks::Hooks>,
}

impl Plugin for Weecord {
    fn init(_: &Weechat, _: Args) -> StdResult<Self, ()> {
        let config = config::Config::new();
        if config.read().is_err() {
            return Err(());
        }

        Ok(Weecord {
            discord_connection: DiscordConnection::new(),
            config,
            instance: Instance::new(),
            hooks: None,
        })
    }

    fn ready(&mut self, weechat: &Weechat) {
        if let Err(err) = self.setup_tracing() {
            Weechat::print(&format!(
                "discord: Unable to setup logging, trace window will be empty!: {}",
                err
            ));
        }

        if self.config.auto_open_tracing() {
            buffer::debug::Debug::create_buffer();
        }

        if let Some(token) = self.config.token() {
            let (tx, rx) = channel(1000);

            Weechat::spawn({
                let discord_connection = self.discord_connection.clone();
                let config = self.config.clone();
                let instance = self.instance.clone();
                async move {
                    if let Ok(connection) = discord_connection.start(&token, tx).await {
                        DiscordConnection::handle_events(rx, &connection, config, instance).await;
                    }
                }
            })
            .detach();
        };

        self.hooks.replace(hooks::Hooks::hook_all(
            weechat,
            self.discord_connection.clone(),
            self.instance.clone(),
            self.config.clone(),
        ));
    }
}

impl Weecord {
    fn env_filter(&self) -> EnvFilter {
        let mut env_filter = EnvFilter::new(self.config.log_directive())
            // Set the default log level to warn
            .add_directive(LevelFilter::WARN.into());
        // Allow `WEECORD_LOG` env to override
        for directive in std::env::var("WEECORD_LOG")
            .unwrap_or_default()
            .split(',')
            .flat_map(|d| d.parse().ok())
        {
            env_filter = env_filter.add_directive(directive);
        }
        env_filter
    }

    #[cfg(not(feature = "tracing_tree"))]
    fn setup_tracing(&self) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        tracing_subscriber::fmt()
            .with_env_filter(self.env_filter())
            .with_writer(move || buffer::debug::Debug)
            .without_time()
            .try_init()
    }

    #[cfg(feature = "tracing_tree")]
    fn setup_tracing(&self) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        use tracing_subscriber::{layer::SubscriberExt, Layer, Registry};
        let subscriber = Registry::default().with(
            tracing_tree::HierarchicalLayer::new(2)
                .with_ansi(true)
                .with_writer(move || buffer::debug::Debug)
                .and_then(self.env_filter()),
        );
        tracing::subscriber::set_global_default(subscriber).map_err(|err| err.into())
    }
}

impl Drop for Weecord {
    fn drop(&mut self) {
        // Ensure all buffers are cleared
        // Drain each map first so we can drop the mut map borrow so that the close handler doesn't
        // deadlock
        let mut pins: Vec<_> = self.instance.borrow_pins_mut().drain().collect();
        pins.clear();
        let mut channels: Vec<_> = self.instance.borrow_guilds_mut().drain().collect();
        channels.clear();
        let mut guilds: Vec<_> = self.instance.borrow_guilds_mut().drain().collect();
        guilds.clear();
        // Prevent any further traces from being printed (causes segfaults)
        SHUTTING_DOWN.trigger();
        tracing::trace!("Plugin unloaded");
    }
}

plugin!(
    Weecord,
    name: "weecord",
    author: "Noskcaj19",
    description: "Discord integration for weechat",
    version: "0.3.0",
    license: "MIT"
);
