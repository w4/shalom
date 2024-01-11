use std::num::NonZeroUsize;

use iced::{futures::stream, subscription, widget::image, Subscription};
use lru::LruCache;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use reqwest::IntoUrl;
use url::Url;

use crate::config::FANART_PROJECT_KEY;

#[derive(Debug)]
pub enum MaybePendingImage {
    Downloaded(image::Handle),
    Loading(Url),
}

impl MaybePendingImage {
    pub fn handle(&self) -> Option<image::Handle> {
        match self {
            Self::Downloaded(h) => Some(h.clone()),
            Self::Loading(_) => None,
        }
    }
}

pub fn download_image<M: 'static>(
    url: Url,
    post_process: fn(::image::RgbaImage) -> ::image::RgbaImage,
    resp: impl FnOnce(image::Handle) -> M + Send + 'static,
) -> Subscription<M> {
    subscription::run_with_id(
        url.to_string(),
        stream::once(async move {
            eprintln!("{url} dl");

            (resp)(load_image(url.clone(), post_process).await)
        }),
    )
}

pub async fn load_image<T: IntoUrl>(
    url: T,
    post_process: fn(::image::RgbaImage) -> ::image::RgbaImage,
) -> image::Handle {
    static CACHE: Lazy<Mutex<LruCache<Url, image::Handle>>> =
        Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(50).unwrap())));

    let url = url.into_url().unwrap();

    if let Some(handle) = CACHE.lock().get(&url) {
        return handle.clone();
    }

    let bytes = reqwest::get(url.clone())
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();

    let handle = tokio::task::spawn_blocking(move || {
        eprintln!("parsing image");
        let img = ::image::load_from_memory(&bytes).unwrap();
        eprintln!("post processing");
        let data = post_process(img.into_rgba8());
        let (h, w) = data.dimensions();
        image::Handle::from_pixels(h, w, data.into_raw())
    })
    .await
    .unwrap();

    CACHE.lock().push(url.clone(), handle.clone());

    handle
}

pub fn find_musicbrainz_artist<M: 'static>(
    artist: String,
    to_msg: fn(String) -> M,
) -> Subscription<M> {
    static CACHE: Lazy<Mutex<LruCache<String, String>>> =
        Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(10).unwrap())));

    subscription::run_with_id(
        format!("musicbrainz-{artist}"),
        stream::once(async move {
            eprintln!("musicbrainz req");

            if let Some(handle) = CACHE.lock().get(&artist) {
                return (to_msg)(handle.to_string());
            }

            // TODO
            let client = reqwest::Client::builder()
                .user_agent(format!(
                    "{}/{}",
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION")
                ))
                .build()
                .unwrap();

            let resp: serde_json::Value = client
                .get(format!(
                    "https://musicbrainz.org/ws/2/artist/?query={artist}&fmt=json",
                ))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();

            let id = resp
                .get("artists")
                .unwrap()
                .get(0)
                .unwrap()
                .get("id")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();

            CACHE.lock().push(artist, id.clone());

            // TODO: typing
            (to_msg)(id)
        }),
    )
}

pub fn find_fanart_urls<M: 'static>(
    musicbrainz_id: String,
    to_msg: fn(Option<Url>, Option<Url>) -> M,
) -> Subscription<M> {
    subscription::run_with_id(
        format!("fanart-{musicbrainz_id}"),
        stream::once(async move {
            eprintln!("fanart req");

            let resp: serde_json::Value = reqwest::get(format!("http://webservice.fanart.tv/v3/music/{musicbrainz_id}?api_key={FANART_PROJECT_KEY}"))
                .await
                .unwrap()
                .json()
                .await
                .unwrap();

            // TODO: typing
            let logo = resp
                .get("hdmusiclogo")
                .and_then(|v| v.get(0))
                .and_then(|v| v.get("url"))
                .and_then(|v| v.as_str())
                .map(Url::parse)
                .transpose()
                .unwrap();
            let background = resp
                .get("artistbackground")
                .and_then(|v| v.get(0))
                .and_then(|v| v.get("url"))
                .and_then(|v| v.as_str())
                .map(Url::parse)
                .transpose()
                .unwrap();

            (to_msg)(logo, background)
        }),
    )
}
