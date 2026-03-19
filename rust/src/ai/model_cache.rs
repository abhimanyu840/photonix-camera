use anyhow::Result;
use once_cell::sync::Lazy;
use ort::session::Session;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const MAX_CACHED: usize = 2;

pub const MODEL_KEY_SCENE: &str = "scene_cls";
pub const MODEL_KEY_DENOISER: &str = "denoiser";
pub const MODEL_KEY_ENHANCER: &str = "enhancer";
pub const MODEL_KEY_SUPER_RES: &str = "super_res";
pub const MODEL_KEY_DEPTH: &str = "depth";

struct Entry {
    key: String,
    session: Arc<Mutex<Session>>,
}

struct Cache {
    pinned: Option<Arc<Mutex<Session>>>,
    lru: VecDeque<Entry>,
    paths: std::collections::HashMap<String, PathBuf>,
}

impl Cache {
    fn new() -> Self {
        Self {
            pinned: None,
            lru: VecDeque::new(),
            paths: Default::default(),
        }
    }

    fn get(&mut self, key: &str) -> Option<Arc<Mutex<Session>>> {
        if key == MODEL_KEY_SCENE {
            return self.pinned.clone();
        }
        if let Some(idx) = self.lru.iter().position(|e| e.key == key) {
            let e = self.lru.remove(idx).unwrap();
            let s = e.session.clone();
            self.lru.push_front(e);
            return Some(s);
        }
        None
    }

    fn insert(&mut self, key: &str, s: Arc<Mutex<Session>>) {
        if key == MODEL_KEY_SCENE {
            self.pinned = Some(s);
            return;
        }
        if self.lru.len() >= MAX_CACHED {
            self.lru.pop_back();
        }
        self.lru.push_front(Entry {
            key: key.to_string(),
            session: s,
        });
    }
}

static CACHE: Lazy<Mutex<Cache>> = Lazy::new(|| Mutex::new(Cache::new()));

pub fn register_models(paths: &[(&str, &str)]) {
    let mut c = CACHE.lock().unwrap();
    for (k, p) in paths {
        c.paths.insert(k.to_string(), PathBuf::from(p));
    }
}

pub fn load_model(key: &str) -> Result<Arc<Mutex<Session>>> {
    if let Some(s) = CACHE.lock().unwrap().get(key) {
        return Ok(s);
    }
    let path = CACHE
        .lock()
        .unwrap()
        .paths
        .get(key)
        .ok_or_else(|| anyhow::anyhow!("Model '{}' not registered", key))?
        .clone();
    log::info!("[Cache] Loading: {}", key);
    let s = Arc::new(Mutex::new(super::session_pool::build_session(&path)?));
    CACHE.lock().unwrap().insert(key, s.clone());
    Ok(s)
}
