use crate::{config::Charset, Weechat2};
use anyhow::Context;
use image::{DynamicImage, GenericImageView, ImageFormat};
use std::borrow::Cow;
use tokio::runtime::Runtime;
use twilight_model::channel::Message;

pub struct InlineImageCandidate {
    pub url: String,
    pub height: u64,
    pub width: u64,
}

pub fn find_image_candidates(msg: &Message) -> Vec<InlineImageCandidate> {
    let mut out = Vec::new();

    for attachment in &msg.attachments {
        if ImageFormat::from_path(&attachment.proxy_url).is_ok() {
            out.push(InlineImageCandidate {
                url: attachment.proxy_url.clone(),
                height: attachment.height.unwrap_or(900),
                width: attachment.width.unwrap_or(20),
            });
        }
    }

    for embed in &msg.embeds {
        if let Some(thumbnail) = embed.thumbnail.as_ref() {
            if let Some(url) = thumbnail.proxy_url.as_ref() {
                out.push(InlineImageCandidate {
                    url: url.clone(),
                    height: thumbnail.height.unwrap_or(900 * 2) / 2,
                    width: thumbnail.width.unwrap_or(20 * 2) / 2,
                });
            }
        }
    }

    out
}

/// Wraps reqwest fetch so this function can be called on a weechat future
pub async fn fetch_inline_image(rt: &Runtime, url: &str) -> anyhow::Result<DynamicImage> {
    let url = url.to_owned();
    rt.spawn(async move {
        tracing::trace!("Fetching inline image at: {}", url);

        let client = hyper::Client::builder().build::<_, hyper::Body>(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_only()
                .enable_http1()
                .enable_http2()
                .build(),
        );

        let uri = url.parse().expect("Discord sent an invalid uri");
        let response = client
            .get(uri)
            .await
            .with_context(|| format!("Failed to fetch image url: {}", url))?;
        let body = hyper::body::to_bytes(response)
            .await
            .context("Failed to fetch image asset body")?;

        tracing::trace!("Successfully loaded image");
        image::load_from_memory(body.as_ref()).context("Failed to load image")
    })
    .await
    .expect("Task is never aborted")
}

pub fn render_img(img: &DynamicImage, charset: Charset) -> String {
    let render = term_image::block::Block::img_exact(
        &term_image::block::BlockOptions {
            char_set: charset,
            blend: true,
            background_color: [0, 0, 0].into(),
            size: (img.width() as u16, img.height() as u16),
        },
        Cow::Borrowed(img),
    );

    let mut out = String::new();

    for y in render.into_iter() {
        for x in y {
            let fg = x.fg.as_256().0;
            let bg = x.bg.as_256().0;
            out.push_str(&format!(
                "{}{}",
                Weechat2::color(&format!("{},{}", fg, bg)),
                x.ch,
            ));

            out.push_str(Weechat2::color("reset"));
        }
        out.push('\n');
    }

    out
}
