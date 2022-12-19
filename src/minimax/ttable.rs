use super::Score;
use super::Move;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use fxhash::FxHasher64;

const TT_LENGTH: usize = 12582917; // prime
#[cfg(not(feature = "training"))]
const MAX_SIMUL_GAMES: usize = 1;
#[cfg(feature = "training")]
const MAX_SIMUL_GAMES: usize = 4; // training requires one TT per snake

/// The transposition table of this battlesnake.
/// Is encapsulated in this module and only accessible via the get and insert functions.
// static mut TABLES: Option<Vec<Vec<Entry>>> = None;
static mut TABLE: Option<Vec<Entry>> = None;

/// The Pair holds the next tt_id to give out and a list of game IDs.
/// The index of a game ID is the tt_id of that game.
static mut GAME_IDS: Option<Mutex<(u8, Vec<String>)>> = None;

/// Initializes an empty transposition table.
pub fn init() {
    unsafe {
        if let None = TABLE {
            TABLE = Some(vec![Entry{data: 0, key: 0}; TT_LENGTH*MAX_SIMUL_GAMES]);
        }
        if let None = GAME_IDS {
            GAME_IDS = Some(Mutex::new((0, vec!["".to_string(); MAX_SIMUL_GAMES])));
        }
    }
    println!("TTables initialized")
}

pub fn get_tt_id(game_id: String) -> u8 {
    unsafe {
        #[cfg(feature = "training")]
        {
            if game_id.len() == 1 {
                if let Ok(x) = game_id.parse() {
                    return x
                }
            }
        }
        if let Some(tmp) = &mut GAME_IDS {
            let mut game_ids = tmp.lock().unwrap();
            for (i, id) in &mut game_ids.1.iter().enumerate() {
                if *id == game_id {
                    return i as u8;
                }
            }
            let tt_id = game_ids.0;
            game_ids.1[tt_id as usize] = game_id;
            game_ids.0 += 1;
            game_ids.0 %= MAX_SIMUL_GAMES as u8;
            tt_id
        } else {
            0
        }
    }
}

fn index(tt_id: u8, key: u64) -> usize {
    tt_id as usize * TT_LENGTH + (key % TT_LENGTH as u64) as usize
}

/// Get an entry from the transposition table
pub fn get(key: u64, tt_id: u8) -> Option<Entry> {
    unsafe {
        if let Some(table) = &TABLE {
            let entry = table[index(tt_id, key)];
            if entry.matches_key(key) {
                return Some(entry)
            }
        }
    }
    None
}

/// Insert an entry into the transposition table
pub fn insert<const S: usize>(
    key: u64,
    tt_id: u8,
    score: Score,
    is_lower_bound: bool,
    is_upper_bound: bool,
    depth: u8,
    best_moves: [Move; S]
) {
    unsafe {
        if let Some(table) = &mut TABLE {
            let index = index(tt_id, key);
            let entry = table[index];
            if entry.matches_key(key) && entry.get_depth() > depth {
                return
            }
            table[index] = Entry::new(key, score, is_lower_bound, is_upper_bound, depth, best_moves);
        }
    }
}

/// The hash function that is used for the transposition table
pub fn hash(board: &impl Hash) -> u64 {
    #[cfg(feature = "tt")]
    {
        let mut hasher = FxHasher64::default();
        board.hash(&mut hasher);
        return hasher.finish()
    }
    #[cfg(not(feature = "tt"))]
    0
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
        let mut data = ((score as u16) as u64) << Self::SCORE_SHIFT // rust cast semantics are annoying here
            | (depth as u64) << Self::DEPTH_SHIFT
            | (is_lower_bound as u64) << Self::LOWER_BOUND_SHIFT
            | (is_upper_bound as u64) << Self::UPPER_BOUND_SHIFT;

        // pack moves in data
        if S <= 8 {
            for i in 0..S {
                data |= (0b_11 & best_moves[i].to_int() as u64) << (Self::BEST_MOVES_SHIFT + i as u32 * Self::MOVE_WIDTH);
            }
        }

        let entry = Entry{key: key ^ data, data};

        debug_assert!(best_moves == entry.get_best_moves::<S>().unwrap(), "IN {:?} OUT {:?}", best_moves, entry.get_best_moves::<S>().unwrap());

        entry
    }

    /// Performs a correctnes check on this entry with a given key.
    /// This is used instead of locking for mostly safe concurrent access.
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

    #[allow(unused)]
    pub fn is_exact(&self) -> bool {
        (self.data >> Self::LOWER_BOUND_SHIFT) & 0b_11 == 0
    }
}
