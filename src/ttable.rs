use crate::types::*;
use crate::bitboard::Bitboard;

use std::hash::{Hash, Hasher};
use fxhash::FxHasher64;

const TT_LENGTH: usize = 1000000;

/// The transposition table of this battlesnake.
/// Is encapsulated in this module and only accessible via the get and insert functions.
static mut TABLE: Option<Vec<Entry>> = None;

/// Initializes the transposition table.
/// Should be called at startup.
pub fn init() {
    unsafe {
        if let None = TABLE {
            TABLE = Some(vec![Entry{data: 0, key: 0}; TT_LENGTH]);
        }
    }
    println!("TTable initialized");
}

/// Get an entry from the transposition table
pub fn get<const S: usize, const W: usize, const H: usize>(board: &Bitboard<S, W, H>) -> Option<Entry>
where [(); (W*H+127)/128]: Sized {
    let key = hash(board);
    let index = key % TT_LENGTH as u64;
    unsafe {
        if let Some(table) = &TABLE {
            let entry = table[index as usize];
            if entry.matches_key(key) {
                return Some(entry)
            }
        }
    }
    None
}

/// Insert an entry into the transposition table
pub fn insert<const S: usize, const W: usize, const H: usize>(board: &Bitboard<S, W, H>, score: Score, depth: u8)
where [(); (W*H+127)/128]: Sized {
    let key = hash(board);
    let index = key % TT_LENGTH as u64;
    unsafe {
        if let Some(table) = &mut TABLE {
            table[index as usize] = Entry::new(key, score, depth);
        }
    }
}

/// The has function that is used for the transposition table
fn hash<const S: usize, const W: usize, const H: usize>(board: &Bitboard<S, W, H>) -> u64
where [(); (W*H+127)/128]: Sized {
    let mut hasher = FxHasher64::default();
    board.hash(&mut hasher);
    hasher.finish()
}

/// A transposition table entry.
/// Data consits of a single 64 bit integer that is completely encapsulated.
/// Most of the available data payload is currently unused.
#[derive(Clone, Copy)]
pub struct Entry {
    key: u64,
    data: u64,
}

impl Entry {
    const DEPTH_SHIFT: u32 = 0;
    const SCORE_SHIFT: u32 = 8;

    const fn new(key: u64, score: Score, depth: u8) -> Self {
        let data = (score as u64) << Self::SCORE_SHIFT | (depth as u64) << Self::DEPTH_SHIFT;
        Entry{key: key ^ data, data}
    }

    /// Performs a correctnes check on this entry with a given key.
    /// This is used instead of locking for concurrent access.
    fn matches_key(&self, key: u64) -> bool {
        self.key != 0
        && self.data != 0
        && self.key ^ self.data == key
    }
    
    pub fn get_depth(&self) -> u8 {
        (self.data >> Self::DEPTH_SHIFT) as u8
    }
    
    pub fn get_score(&self) -> Score {
        (self.data >> Self::SCORE_SHIFT) as Score
    }
}
