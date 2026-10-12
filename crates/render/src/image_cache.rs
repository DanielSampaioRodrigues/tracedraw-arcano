//! Decoded pictures, kept between draws: decoding a large PNG takes far
//! longer than drawing it, and a page draws its pictures again on every
//! change of the view. Pictures are found by a fingerprint of their PNG
//! bytes; past a memory budget the least recently drawn go first.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tiny_skia::Pixmap;

/// Bytes of decoded pixels kept per drawing thread.
#[cfg(target_arch = "wasm32")]
const BUDGET: usize = 256 << 20;
#[cfg(not(target_arch = "wasm32"))]
const BUDGET: usize = 1024 << 20;

#[derive(Default)]
struct Cache {
    /// Fingerprint to (picture, last use).
    map: HashMap<u64, (Arc<Pixmap>, u64)>,
    bytes: usize,
    tick: u64,
}

impl Cache {
    fn get(&mut self, key: u64) -> Option<Arc<Pixmap>> {
        self.tick += 1;
        let tick = self.tick;
        self.map.get_mut(&key).map(|e| {
            e.1 = tick;
            e.0.clone()
        })
    }

    /// Keep `img` under `key`, dropping the least recently used pictures
    /// until the cache fits `budget`; a picture larger than half the
    /// budget is not kept.
    fn insert(&mut self, key: u64, img: Arc<Pixmap>, budget: usize) {
        let size = img.data().len();
        if size > budget / 2 {
            return;
        }
        self.tick += 1;
        if let Some((old, _)) = self.map.insert(key, (img, self.tick)) {
            self.bytes -= old.data().len();
        }
        self.bytes += size;
        while self.bytes > budget {
            let Some((&oldest, _)) = self.map.iter().min_by_key(|(_, (_, t))| *t) else {
                break;
            };
            if let Some((gone, _)) = self.map.remove(&oldest) {
                self.bytes -= gone.data().len();
            }
        }
    }
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

/// A fingerprint of `png`: its length and bytes taken from its start, its
/// end (the last chunks and their checksums) and evenly in between.
fn fingerprint(png: &[u8]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let n = png.len();
    n.hash(&mut h);
    let mut take = |at: usize, len: usize| {
        png.get(at.min(n)..at.saturating_add(len).min(n))
            .hash(&mut h);
    };
    take(0, 64);
    take(n.saturating_sub(64), 64);
    for i in 1..32 {
        take(n / 32 * i, 32);
    }
    h.finish()
}

/// The picture of `png`, decoded once and then kept.
pub(crate) fn decode(png: &[u8]) -> Option<Arc<Pixmap>> {
    let key = fingerprint(png);
    if let Some(hit) = CACHE.with(|c| c.borrow_mut().get(key)) {
        return Some(hit);
    }
    let img = Arc::new(Pixmap::decode_png(png).ok()?);
    CACHE.with(|c| c.borrow_mut().insert(key, img.clone(), BUDGET));
    Some(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32, shade: u8) -> Vec<u8> {
        let mut p = Pixmap::new(w, h).expect("pixmap");
        p.fill(tiny_skia::Color::from_rgba8(shade, 0, 0, 255));
        p.encode_png().expect("png")
    }

    #[test]
    fn a_picture_is_decoded_once() {
        let a = png(4, 4, 10);
        let b = png(4, 4, 200);
        let first = decode(&a).expect("decoded");
        let again = decode(&a).expect("decoded");
        assert!(Arc::ptr_eq(&first, &again));
        let other = decode(&b).expect("decoded");
        assert!(!Arc::ptr_eq(&first, &other));
        assert_eq!(other.pixel(0, 0).map(|p| p.red()), Some(200));
        assert!(decode(b"not a png").is_none());
        assert_ne!(fingerprint(&a), fingerprint(&b));
        assert_ne!(fingerprint(&[]), fingerprint(&[0]));
    }

    #[test]
    fn the_oldest_pictures_go_past_the_budget() {
        let mut c = Cache::default();
        let img = |shade| Arc::new(Pixmap::decode_png(&png(8, 8, shade)).expect("png"));
        // 256 bytes each, a budget of 600: two fit.
        c.insert(1, img(1), 600);
        c.insert(2, img(2), 600);
        assert!(c.get(1).is_some()); // 1 is now newer than 2
        c.insert(3, img(3), 600);
        assert!(c.get(2).is_none());
        assert!(c.get(1).is_some() && c.get(3).is_some());
        assert_eq!(c.bytes, 512);
        // Too large for the budget: not kept.
        c.insert(4, img(4), 300);
        assert!(c.get(4).is_none());
    }
}
