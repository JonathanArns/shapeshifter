use crate::types::*;

use std::hash::{Hash, Hasher};
use fxhash::FxHasher64;

const TT_LENGTH: usize = 10000000;

/// The transposition table of this battlesnake.
/// Is encapsulated in this module and only accessible via the get and insert functions.
static mut TABLE: Option<Vec<Entry>> = None;

// The debug table holds a hash value for each entry that was computed using a different
// hash function. This is used to detect key collisions in debug mode.
#[cfg(feature = "detect_hash_collisions")]
static mut DEBUG_TABLE: Option<Vec<u64>> = None;

/// Initializes the transposition table.
/// Should be called at startup.
pub fn init() {
    unsafe {
        if let None = TABLE {
            TABLE = Some(vec![Entry{data: 0, key: 0}; TT_LENGTH]);
        }

        #[cfg(feature = "detect_hash_collisions")]
        if let None = DEBUG_TABLE {
            DEBUG_TABLE = Some(vec![0; TT_LENGTH]);
        }
    }
    println!("TTable initialized");
}

pub fn clear() {
    unsafe {
        TABLE = Some(vec![Entry{data: 0, key: 0}; TT_LENGTH]);
    }
}

/// Get an entry from the transposition table
pub fn get(board: &impl Hash) -> Option<Entry> {
    let key = hash(board);
    let index = key % TT_LENGTH as u64;


    unsafe {
        if let Some(table) = &TABLE {
            let entry = table[index as usize];
            if entry.matches_key(key) {

                #[cfg(feature = "detect_hash_collisions")]
                {
                    let debug_hash_val = debug_hash(board);
                    if let Some(table) = &DEBUG_TABLE {
                        let debug_val = table[index as usize];
                        if debug_val != debug_hash_val {
                            println!("TT KEY COLLISION DETECTED!");
                        }
                    }
                }

                return Some(entry)
            }
        }
    }
    None
}

/// Insert an entry into the transposition table
pub fn insert<const S: usize>(
    board: &impl Hash,
    score: Score,
    is_lower_bound: bool,
    is_upper_bound: bool,
    depth: u8,
    best_moves: [Move; S]
) {
    let key = hash(board);
    let index = key % TT_LENGTH as u64;
    unsafe {
        if let Some(table) = &mut TABLE {
            table[index as usize] = Entry::new(key, score, is_lower_bound, is_upper_bound, depth, best_moves);
        }
    }

    #[cfg(feature = "detect_hash_collisions")]
    unsafe {
        if let Some(table) = &mut DEBUG_TABLE {
            table[index as usize] = debug_hash(board);
        }
    }
}

/// The hash function that is used for the transposition table
fn hash(board: &impl Hash) -> u64 {
    let mut hasher = FxHasher64::default();
    board.hash(&mut hasher);
    hasher.finish()
}

/// The hash function that is used for the debug table
fn debug_hash(board: &impl Hash) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
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
    const BEST_MOVES_SHIFT: u32 = 24;
    const MOVE_WIDTH: u32 = 2;
    const LOWER_BOUND_SHIFT: u32 = 40;
    const UPPER_BOUND_SHIFT: u32 = 41;
    // const NEXT_FREE_SHIFT: u32 = 42;

    fn new<const S: usize>(
        key: u64,
        score: Score,
        is_lower_bound: bool,
        is_upper_bound: bool,
        depth: u8,
        best_moves: [Move; S],
    ) -> Self {
        let mut data = (score as u64) << Self::SCORE_SHIFT
            | (depth as u64) << Self::DEPTH_SHIFT
            | (is_lower_bound as u64) << Self::LOWER_BOUND_SHIFT
            | (is_upper_bound as u64) << Self::UPPER_BOUND_SHIFT;

        // pack moves in data
        if S <= 8 {
            for i in 0..S {
                data |= (0b_11 & best_moves[i].to_int() as u64) << (Self::BEST_MOVES_SHIFT + i as u32 * Self::MOVE_WIDTH);
            }
        }
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

    pub fn get_best_moves<const S: usize>(&self) -> Option<[Move; S]> {
        if S > 8 {
            return None
        }
        let mut moves = [Move::Up; S];
        for i in 0..S {
            moves[i] = Move::from_int(0b_11 & (self.data >> Self::BEST_MOVES_SHIFT + i as u32 * Self::MOVE_WIDTH) as u8);
        }
        Some(moves)
    }

    pub fn is_lower_bound(&self) -> bool {
        (self.data >> Self::LOWER_BOUND_SHIFT) & 1 != 0
    }

    pub fn is_upper_bound(&self) -> bool {
        (self.data >> Self::UPPER_BOUND_SHIFT) & 1 != 0
    }

    pub fn is_exact(&self) -> bool {
        (self.data >> Self::LOWER_BOUND_SHIFT) & 0b_11 == 0
    }
}
