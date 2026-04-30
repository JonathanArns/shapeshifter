use crate::bitboard::{bitset::BitsetTrait, constants};

pub trait Mode: Sized + Clone + Send + 'static {
    const W: usize;
    const H: usize;
    const WRAP: bool;

    const N: usize;
    const L: usize;
    type Bitset: BitsetTrait;

    const FULL_BOARD_MASK: Self::Bitset;
    const CHECKER_BOARD_MASK: Self::Bitset;
    const ALL_BUT_LEFT_EDGE_MASK: Self::Bitset;
    const ALL_BUT_RIGHT_EDGE_MASK: Self::Bitset;
    const TOP_EDGE_MASK: Self::Bitset;
    const BOTTOM_EDGE_MASK: Self::Bitset;
    const LEFT_EDGE_MASK: Self::Bitset;
    const RIGHT_EDGE_MASK: Self::Bitset;

    // An opaque type, used for moves_from_position.
    type MoveTable;
    const MOVE_TABLE: Self::MoveTable;
    fn moves_from_position(pos: u16) -> [Option<u16>; 4];

    // TODO: generate the implementations of this via a macro
    // TODO: move the extra stuff in attach_rules here?
}

const fn hz_stack_len(w: usize, h: usize, stack: bool) -> usize {
    if stack {
        w * h
    } else {
        0
    }
}

#[derive(Clone)]
pub struct Standard{}
impl Mode for Standard {
    const W: usize = 11;
    const H: usize = 11;
    const WRAP: bool = false;

    const N: usize = Self::W * Self::H;
    const L: usize = (Self::W*Self::H+63)/64;
    type Bitset = crate::bitboard::bitset::Bitset<{Self::N}, {Self::L}>;
        
    const FULL_BOARD_MASK: Self::Bitset = Self::Bitset::with_all_bits_set();
    const CHECKER_BOARD_MASK: Self::Bitset = constants::checker_board_mask::<{Self::N}, {Self::L}>(Self::W, Self::H);
    const ALL_BUT_LEFT_EDGE_MASK: Self::Bitset = constants::border_mask::<{Self::N}, {Self::L}>(Self::W, Self::H, true);
    const ALL_BUT_RIGHT_EDGE_MASK: Self::Bitset = constants::border_mask::<{Self::N}, {Self::L}>(Self::W, Self::H, false);
    const TOP_EDGE_MASK: Self::Bitset = constants::horizontal_edge_mask::<{Self::N}, {Self::L}>(Self::W, Self::H, true);
    const BOTTOM_EDGE_MASK: Self::Bitset = constants::horizontal_edge_mask::<{Self::N}, {Self::L}>(Self::W, Self::H, false);
    const LEFT_EDGE_MASK: Self::Bitset = constants::vertical_edge_mask::<{Self::N}, {Self::L}>(Self::W, Self::H, true);
    const RIGHT_EDGE_MASK: Self::Bitset = constants::vertical_edge_mask::<{Self::N}, {Self::L}>(Self::W, Self::H, false);

    type MoveTable = [[Option<u16>; 4]; Self::N];
    const MOVE_TABLE: Self::MoveTable = constants::precompute_moves::<{Self::N}, {Self::L}>(Self::W, Self::H, Self::WRAP);

    fn moves_from_position(pos: u16) -> [Option<u16>; 4] {
        Self::MOVE_TABLE[pos as usize]
    }
}

#[derive(Clone)]
pub struct StandardWrapped{}
impl Mode for StandardWrapped {
    const W: usize = 11;
    const H: usize = 11;
    const WRAP: bool = true;

    const N: usize = Self::W * Self::H;
    const L: usize = (Self::W*Self::H+63)/64;
    type Bitset = crate::bitboard::bitset::Bitset<{Self::N}, {Self::L}>;
        
    const FULL_BOARD_MASK: Self::Bitset = Self::Bitset::with_all_bits_set();
    const CHECKER_BOARD_MASK: Self::Bitset = constants::checker_board_mask::<{Self::N}, {Self::L}>(Self::W, Self::H);
    const ALL_BUT_LEFT_EDGE_MASK: Self::Bitset = constants::border_mask::<{Self::N}, {Self::L}>(Self::W, Self::H, true);
    const ALL_BUT_RIGHT_EDGE_MASK: Self::Bitset = constants::border_mask::<{Self::N}, {Self::L}>(Self::W, Self::H, false);
    const TOP_EDGE_MASK: Self::Bitset = constants::horizontal_edge_mask::<{Self::N}, {Self::L}>(Self::W, Self::H, true);
    const BOTTOM_EDGE_MASK: Self::Bitset = constants::horizontal_edge_mask::<{Self::N}, {Self::L}>(Self::W, Self::H, false);
    const LEFT_EDGE_MASK: Self::Bitset = constants::vertical_edge_mask::<{Self::N}, {Self::L}>(Self::W, Self::H, true);
    const RIGHT_EDGE_MASK: Self::Bitset = constants::vertical_edge_mask::<{Self::N}, {Self::L}>(Self::W, Self::H, false);

    type MoveTable = [[Option<u16>; 4]; Self::N];
    const MOVE_TABLE: Self::MoveTable = constants::precompute_moves::<{Self::N}, {Self::L}>(Self::W, Self::H, Self::WRAP);

    fn moves_from_position(pos: u16) -> [Option<u16>; 4] {
        Self::MOVE_TABLE[pos as usize]
    }
}
