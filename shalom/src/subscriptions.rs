use std::{hash::Hash, num::NonZeroUsize};

use iced::{futures::stream, subscription, widget::image, Subscription};
use lru::LruCache;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use url::Url;

pub fn download_image<I: Hash + Copy + Send + 'static, M: 'static>(
    id: I,
    url: Url,
    resp: fn(I, Url, image::Handle) -> M,
) -> Subscription<M> {
    static CACHE: Lazy<Mutex<LruCache<Url, image::Handle>>> =
        Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(10).unwrap())));

    subscription::run_with_id(
        id,
        stream::once(async move {
            if let Some(handle) = CACHE.lock().get(&url) {
                return (resp)(id, url, handle.clone());
            }

            let bytes = reqwest::get(url.clone())
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap();
            let handle = image::Handle::from_memory(bytes);

            CACHE.lock().push(url.clone(), handle.clone());

            (resp)(id, url, handle)
        }),
    )
}
