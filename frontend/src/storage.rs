// Storage utility for local storage access.
//
// Provides unified read/write methods with quote stripping.
//
// Copyright (c) 2026 RustDrop Authors. All rights reserved.

pub struct StorageService;

impl StorageService {
    fn local_storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    pub fn get_item(key: &str, default: &str) -> String {
        let val = Self::local_storage().and_then(|s| s.get_item(key).ok().flatten());
        match val {
            Some(v) => {
                if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
                    let clean = v[1..v.len() - 1].to_string();
                    Self::set_item(key, &clean);
                    clean
                } else {
                    v
                }
            }
            None => default.to_string(),
        }
    }

    pub fn set_item(key: &str, value: &str) {
        if let Some(s) = Self::local_storage() {
            let _ = s.set_item(key, value);
        }
    }
}
