use crate::bitboard::Bitset;

/// Computes ALL_BUT_LEFT_EDGE_MASK and ALL_BUT_RIGHT_EDGE_MASK
pub const fn border_mask<const N: usize, const L: usize>(w: usize, h: usize, left: bool) -> Bitset<N, L>
where [(); L]: Sized {
    let mut arr = [0_u64; L];
    let mut i = 0;
    let mut j;
    loop {
        if i == h {
            break
        }
        if left {
            j = 0;
        } else {
            j = 1;
        }
        loop {
            if left && j == w-1 {
                break
            } else if !left && j == w {
                break
            }
            let idx = (i*w+j)>>6;
            let offset = (i*w+j) % 64;
            arr[idx] |= 1_u64<<offset;

            j += 1;
        }
        i += 1;
    }
    Bitset::<N, L>::from_array(arr)
}

/// Computes ALL_BUT_LEFT_EDGE_MASK and ALL_BUT_RIGHT_EDGE_MASK
pub const fn checker_board_mask<const N: usize, const L: usize>(w: usize, h: usize) -> Bitset<N, L>
where [(); L]: Sized {
    let mut arr = [0_u64; L];
    let mut i = 0;
    let mut j;
    loop {
        if i == h {
            break
        }
        j = 0;
        loop {
            if j == w {
                break
            }
            if (i*w+j) % 2 == 0 {
                let idx = (i*w+j)>>6;
                let offset = (i*w+j) % 64;
                arr[idx] |= 1_u64<<offset;
            }
            j += 1;
        }
        i += 1;
    }
    Bitset::<N, L>::from_array(arr)
}

/// Computes LEFT_EDGE_MASK and RIGHT_EDGE_MASK
pub const fn vertical_edge_mask<const N: usize, const L: usize>(w: usize, h: usize, right: bool) -> Bitset<N, L>
where [(); L]: Sized {
    let mut arr = [0_u64; L];
    let mut i = 0;
    let j = if right { w-1 } else { 0 };
    loop {
        if i == h {
            break
        }
        let idx = (i*w+j)>>6;
        let offset = (i*w+j) % 64;
        arr[idx] |= 1_u64<<offset;
        i += 1;
    }
    Bitset::<N, L>::from_array(arr)
}

/// Computes TOP_EDGE_MASK and BOTTOM_EDGE_MASK
pub const fn horizontal_edge_mask<const N: usize, const L: usize>(w: usize, h: usize, top: bool) -> Bitset<N, L>
where [(); L]: Sized {
    let mut arr = [0_u64; L];
    let i = if top { h-1 } else { 0 };
    let mut j = 0;
    loop {
        if j == w {
            break
        }
        let idx = (i*w+j)>>6;
        let offset = (i*w+j) % 64;
        arr[idx] |= 1_u64<<offset;
        j += 1;
    }
    Bitset::<N, L>::from_array(arr)
}

/// Computes possible moves from every position at compile time
pub const fn precompute_moves<const N: usize, const L: usize> (w: usize, h: usize, wrap: bool) -> [[Option<u16>; 4]; N]
where [(); L]: Sized, [(); N]: Sized {
    let mut result = [[None; 4]; {N}];
    let mut pos = 0;
    loop {
        if pos == N {
            break
        }
        if wrap {
            // up
            let move_to = (pos + w) % (w*h);
            result[pos][0] = Some(move_to as u16);
            
            // down
            let move_to = if w > pos { w*(h-1) + pos } else { pos - w };
            result[pos][1] = Some(move_to as u16);
            
            // right
            let move_to = if pos % w == w-1 { pos - (w-1) } else { pos + 1};
            result[pos][2] = Some(move_to as u16);
            
            // left
            let move_to = if pos % w == 0 { pos + (w-1) } else { pos - 1 };
            result[pos][3] = Some(move_to as u16);
        } else {
            // up
            if pos < w * (h-1) {
                let move_to = pos + w;
                result[pos][0] = Some(move_to as u16);
            }
            // down
            if pos >= w {
                let move_to = pos - w;
                result[pos][1] = Some(move_to as u16);
            }
            // right
            if pos % w < w - 1 {
                let move_to = pos + 1;
                result[pos][2] = Some(move_to as u16);
            }
            // left
            if pos % w > 0 {
                let move_to = pos - 1;
                result[pos][3] = Some(move_to as u16);
            }
        }

        pos += 1;
    }
    result
}

pub const fn precompute_hazard_spiral() -> [(i8, i8); 144] {
    [ (0,0), (0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1), (-1, 2), (0, 2), (1, 2), (2, 2), (2, 1), (2, 0), (2, -1), (2, -2), (1, -2), (0, -2), (-1, -2), (-2, -2), (-2, -1), (-2, 0), (-2, 1), (-2, 2), (-2, 3), (-1, 3), (0, 3), (1, 3), (2, 3), (3, 3), (3, 2), (3, 1), (3, 0), (3, -1), (3, -2), (3, -3), (2, -3), (1, -3), (0, -3), (-1, -3), (-2, -3), (-3, -3), (-3, -2), (-3, -1), (-3, 0), (-3, 1), (-3, 2), (-3, 3), (-3, 4), (-2, 4), (-1, 4), (0, 4), (1, 4), (2, 4), (3, 4), (4, 4), (4, 3), (4, 2), (4, 1), (4, 0), (4, -1), (4, -2), (4, -3), (4, -4), (3, -4), (2, -4), (1, -4), (0, -4), (-1, -4), (-2, -4), (-3, -4), (-4, -4), (-4, -3), (-4, -2), (-4, -1), (-4, 0), (-4, 1), (-4, 2), (-4, 3), (-4, 4), (-4, 5), (-3, 5), (-2, 5), (-1, 5), (0, 5), (1, 5), (2, 5), (3, 5), (4, 5), (5, 5), (5, 4), (5, 3), (5, 2), (5, 1), (5, 0), (5, -1), (5, -2), (5, -3), (5, -4), (5, -5), (4, -5), (3, -5), (2, -5), (1, -5), (0, -5), (-1, -5), (-2, -5), (-3, -5), (-4, -5), (-5, -5), (-5, -4), (-5, -3), (-5, -2), (-5, -1), (-5, 0), (-5, 1), (-5, 2), (-5, 3), (-5, 4), (-5, 5), (-5, 6), (-4, 6), (-3, 6), (-2, 6), (-1, 6), (0, 6), (1, 6), (2, 6), (3, 6), (4, 6), (5, 6), (6, 6), (6, 5), (6, 4), (6, 3), (6, 2), (6, 1), (6, 0), (6, -1), (6, -2), (6, -3), (6, -4), (6, -5) ]
}
