use std::{hash::Hash, num::NonZeroUsize, sync::Mutex};

use iced::{futures::stream, subscription, widget::image, Subscription};
use lru::LruCache;
use once_cell::sync::Lazy;
use url::Url;

pub fn download_image<I: Hash + 'static, M: 'static>(
    id: I,
    url: Url,
    resp: fn(Url, image::Handle) -> M,
) -> Subscription<M> {
    static CACHE: Lazy<Mutex<LruCache<Url, image::Handle>>> =
        Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(10).unwrap())));

    subscription::run_with_id(
        id,
        stream::once(async move {
            if let Some(handle) = CACHE.lock().unwrap().get(&url) {
                return (resp)(url, handle.clone());
            }

            let bytes = reqwest::get(url.clone())
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap();
            let handle = image::Handle::from_memory(bytes);

            CACHE.lock().unwrap().push(url.clone(), handle.clone());

            (resp)(url, handle)
        }),
    )
}
