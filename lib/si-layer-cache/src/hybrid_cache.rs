use foyer::{
    DirectFsDeviceOptions, Engine, HybridCache, HybridCacheBuilder, RateLimitPicker, RecoverMode,
    TracingOptions,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use telemetry::tracing::error;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::db::serialize;
use crate::error::LayerDbResult;
use crate::LayerDbError;

const SYSTEM_RESERVED_MEMORY_BYTES: u64 = 1024 * 1024 * 512; // 512mb
const DEFAULT_TAIL_TRACING_THRESHOLD: Duration = Duration::from_millis(500);
const DEFAULT_DISK_CACHE_RATE_LIMIT: usize = 1024 * 1024 * 100;

#[derive(Clone, Debug, Deserialize, Serialize)]
enum MaybeDeserialized<V>
where
    V: Serialize + Clone + Send + Sync + 'static,
{
    RawBytes(Vec<u8>),
    DeserializedValue(V),
}

#[derive(Clone, Debug)]
pub struct Cache<V>
where
    V: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    cache: HybridCache<Arc<str>, MaybeDeserialized<V>>,
}

impl<V> Cache<V>
where
    V: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    pub async fn new(config: CacheConfig) -> LayerDbResult<Self> {
        let cache = HybridCacheBuilder::new()
            .with_name(&config.name)
            .with_tracing_options(
                TracingOptions::new()
                    .with_record_hybrid_obtain_threshold(config.telemetry_tail_duration)
                    .with_record_hybrid_insert_threshold(config.telemetry_tail_duration),
            )
            .memory(config.memory as usize)
            .storage(Engine::Large)
            .with_admission_picker(Arc::new(RateLimitPicker::new(
                config.disk_admission_rate_limit,
            )))
            .with_device_options(DirectFsDeviceOptions::new(config.disk))
            .with_recover_mode(RecoverMode::Quiet)
            .build()
            .await
            .map_err(|e| LayerDbError::Foyer(e.into()))?;

        cache.enable_tracing();
        dbg!(&cache);
        Ok(Self { cache })
    }

    pub async fn get(&self, key: &str) -> Option<V> {
        match self.cache.obtain(key.into()).await {
            Ok(Some(entry)) => match entry.value() {
                // todo: bad clone here
                MaybeDeserialized::DeserializedValue(v) => Some(v.clone()),
                MaybeDeserialized::RawBytes(bytes) => {
                    // If we fail to deserialize the raw bytes for some reason, pretend that we never
                    // had the key in the first place, and also remove it from the cache.
                    match serialize::from_bytes_async::<V>(bytes).await {
                        Ok(deserialized) => {
                            self.insert(key.into(), deserialized.clone());
                            Some(deserialized)
                        }
                        Err(e) => {
                            error!(
                        "Failed to deserialize stored bytes from memory cache for key ({:?}): {}",
                        key,
                        e
                    );
                            self.remove(key);
                            None
                        }
                    }
                }
            },

            _ => None,
        }
    }

    pub fn insert(&self, key: Arc<str>, value: V) {
        self.cache
            .insert(key, MaybeDeserialized::DeserializedValue(value));
    }

    pub fn insert_raw_bytes(&self, key: Arc<str>, raw_bytes: Vec<u8>) {
        self.cache
            .insert(key, MaybeDeserialized::RawBytes(raw_bytes));
    }

    pub fn remove(&self, key: &str) {
        self.cache.remove(key);
    }

    pub fn contains(&self, key: &str) -> bool {
        self.cache.contains(key)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CacheConfig {
    disk: PathBuf,
    disk_admission_rate_limit: usize,
    memory: u64,
    name: String,
    telemetry_tail_duration: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        let sys = sysinfo::System::new_all();

        Self {
            disk: "/tmp/layer_cache".into(),
            disk_admission_rate_limit: DEFAULT_DISK_CACHE_RATE_LIMIT,
            memory: sys.total_memory() - SYSTEM_RESERVED_MEMORY_BYTES, //reserve for OS
            name: "default".to_string(),
            telemetry_tail_duration: DEFAULT_TAIL_TRACING_THRESHOLD,
        }
    }
}
