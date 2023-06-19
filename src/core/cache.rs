use std::{
  sync::{Arc, OnceLock},
  collections::HashMap,
  time::{SystemTime, UNIX_EPOCH},
};

use tokio::sync::{RwLock, RwLockWriteGuard};


pub static MAX_AGE: u64 = 86400;
pub static MAX_ITEM: usize = 1000;
pub static ACCESS_TOKEN_CACHE: OnceLock<AccessTokenCache> = OnceLock::new();


#[derive(Debug, Clone)]
pub struct CacheData {
  pub access_token: Arc<str>,
  pub access_token_secret: Arc<str>,
  cached_at: u64,
}

unsafe impl Send for CacheData {}

impl CacheData {
  fn new(
    access_token: Arc<str>,
    access_token_secret: Arc<str>
  ) -> CacheData {
    CacheData {
      access_token,
      access_token_secret,
      cached_at: SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Epoch fail!")
        .as_secs(),
    }
  }
}

#[derive(Debug)]
pub struct AccessTokenCache {
  data: RwLock<HashMap<u64, CacheData>>,
  max_age: u64
}

unsafe impl Send for AccessTokenCache {}

impl AccessTokenCache {
  pub(self) fn new(max_age: u64, max_item: usize) -> AccessTokenCache {
    AccessTokenCache {
      data: RwLock::new(HashMap::with_capacity(max_item)),
      max_age
    }
  }

  pub fn get() -> &'static AccessTokenCache {
    ACCESS_TOKEN_CACHE.get_or_init(
      || AccessTokenCache::new(MAX_AGE, MAX_ITEM)
    )
  }

  pub async fn request(self: &Self, user_id: u64) -> Option<CacheData> {
    match self.data.read().await.get(&user_id) {
      Some(cache_data) => Some(cache_data.clone()),
      None => None
    }
  }

  pub async fn add(
    self: &Self,
    user_id: u64,
    access_token: Arc<str>,
    access_token_secret: Arc<str>
  ) {
    self.data
      .write()
      .await
      .insert(
        user_id,
        CacheData::new(access_token, access_token_secret)
      );
  }

  pub async fn clean_up(self: &Self) {
    let current_time: u64 = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("Epoch fail!")
      .as_secs();

    let mut cache: RwLockWriteGuard<'_, HashMap<u64, CacheData>> = self.data.write().await;

    cache.retain(
      |_, cache_data: &mut CacheData| {
        (current_time - cache_data.cached_at) <= self.max_age
      }
    );
    cache.shrink_to(MAX_ITEM);
  }
}
