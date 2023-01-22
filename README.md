# Weechat Discord

---

## Warning

***Use at your own risk***: Using this program violates the Discord TOS and could get your account or ip disabled, banned, etc.

[Read more details here](https://github.com/terminal-discord/weechat-discord/wiki/Discord-TOS-and-self-tokens)

I personally use weechat-discord with my alt account for testing and personal use and have not been banned _yet_.

---

[![CI](https://github.com/terminal-discord/weechat-discord/workflows/CI/badge.svg)](https://github.com/terminal-discord/weechat-discord/actions)
[![Discord](https://img.shields.io/discord/715036059712356372?label=discord&logo=discord&logoColor=white)](https://discord.gg/BcPku6R)


A plugin that adds Discord support to [Weechat](https://weechat.org/)

---

* [Installation](#installation)
  * [Building](#building)
  * [Optional Features](#optional-features)
* [Setup](#setup)
* [Configuration](#configuration)
  * [Bar items](#bar-items)
  * [Useful options](#useful-options)
* [Usage](#usage)
  * [Editing](#editing)
* [Note for macOS](#macos)
* [Contributing](#contributing)


### Installation

Binaries are automatically compiled for macOS and linux and archived on [Github Actions](https://terminal-discord.vercel.app/api/latest-build?repo=weechat-discord&workflow=1329556&branch=mk3&redirect)

On macOS you will need to [adjust the Weechat plugin file extensions](#macos)

#### Building

In order to build weechat-discord yourself you will need:

* A recent version of [Rust](https://www.rust-lang.org/)
* Weechat developer libraries (optional)
* [libclang](https://rust-lang.github.io/rust-bindgen/requirements.html)

Compiling with Cargo with result in a shared object file `target/release/libweecord.so` (or `.dylib` on macos), which
then needs to be installed to the `plugins/` directory of your weechat home.

This can be done automatically with the project build tool.

```
cargo xtask install
```

Other commands include:

* `cargo xtask test` - Builds and installs in a test directory
* `cargo xtask run` - Builds and installs globally for release
* `cargo xtask fmt` - Builds and formats the repo
* `cargo xtask` - Is the same as `cargo xtask test`

The global weechat home directory defaults to `~/.weechat` and can be changed by setting `WEECHAT_HOME` and the test
dir defaults to `./test_dir/` and can be changed by setting `WEECHAT_TEST_DIR`

#### Weechat headers

By default, the latest `weechat-plugin.h` file is used, however a system file can be used by setting
`WEECHAT_BUNDLED=false` and setting `WEECHAT_PLUGIN_FILE` to the absolute path of your `weechat-plugin.h` file.

#### Optional Features

Certain additional features can be enabled using cargo feature flags:
* `syntax_highlighting` - Enables syntax highlighting for code blocks (uses the amazing [syntect](https://github.com/trishume/syntect))
* `images` - Enable support for rendering images inline

Features can be enabled when using xtask by setting the `WEECORD_FEATURES` environment variable.

All features are enabled for Github Actions builds.

### Setup

You must first obtain a login token.

A Python script [`find_token.py`](find_token.py) is included in this repo which will attempt to find the tokens used by
installed Discord clients (both the webapp and desktop app should be searched).

The script will attempt to use [ripgrep](https://github.com/BurntSushi/ripgrep) if installed to search faster.

If the script fails, you can get the tokens manually.

With the discord app open in your browser:
1) Open Devtools (ctrl+shift+i or cmd+opt+i)
2) Navigate to the Network tab
3) View only WebSockets by clicking "WS" in the inspector bar
4) Reload the page and select the "gateway.discord.gg" connection
5) Navigate to the "Response" tab of the request
6) The first or second message should begin with `{"op":2,"d":{"token":"<your token here>"...`

Once you have your token you can save your token **insecurely** with 

```
/discord token 123456789ABCDEF
```

However, your token will be saved in plaintext in `$WEECHAT_HOME/weecord.conf` so if this file is committed publicly or shared publicly, anyone can access your account (and know the id's of servers and channels you are in).

To prevent this, secure your token using [secure data](https://weechat.org/blog/post/2013/08/04/Secured-data).
If you secured your token as `discord_token` then you would run

```
/discord token ${sec.data.discord_token}
```

Once your token is set, you can reload the plugin with

```
/plugin reload weecord
```

### Configuration

#### Bar items
##### Typing indicator

The bar item `discord_typing` displays the typing status of the current buffer and can be appended to
`weechat.bar.status.items`.


##### Slowmode cooldown

The bar item `discord_slowmode_cooldown` displays the ratelimit time for the current channel.

#### Useful options

* `weecord.general.send_typing` - This must be set to true for others to see when you are typing


### Usage

#### Editing

Messages can be edited and deleted using ed style substitutions.

To edit the previous message:
```
s/foo/bar/
```

To delete the previous message:
```
s///
```

To select an older message, an offset can be included, for example, to delete the 3rd most recent message (sent by you):
```
3s///
```

### MacOS
Weechat does not search for macos dynamic libraries (.dylib) by default, this can be fixed by adding `.dylib`s to the plugin search path,

```
/set weechat.plugin.extension ".so,.dll,.dylib"
```

## Contributing

PRs are welcome.
Please ensure that the source is formatted by running `cargo xtask fmt` (the code builds on stable but uses nightly rustfmt). 