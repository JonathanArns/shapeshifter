use bitssset::Bitset;

/// Computes ALL_BUT_LEFT_EDGE_MASK and ALL_BUT_RIGHT_EDGE_MASK
pub const fn border_mask<const W: usize, const H: usize>(left: bool) -> Bitset<{W*H}>
where [(); (W*H+63)/64]: Sized {
    let mut arr = [0_u64; (W*H+63)/64];
    let mut i = 0;
    let mut j;
    loop {
        if i == H {
            break
        }
        if left {
            j = 0;
        } else {
            j = 1;
        }
        loop {
            if left && j == W-1 {
                break
            } else if !left && j == W {
                break
            }
            let idx = (i*W+j)>>6;
            let offset = (i*W+j) % 64;
            arr[idx] |= 1_u64<<offset;

            j += 1;
        }
        i += 1;
    }
    Bitset::<{W*H}>::from_array(arr)
}

/// Computes ALL_BUT_LEFT_EDGE_MASK and ALL_BUT_RIGHT_EDGE_MASK
pub const fn checker_board_mask<const W: usize, const H: usize>() -> Bitset<{W*H}>
where [(); (W*H+63)/64]: Sized {
    let mut arr = [0_u64; (W*H+63)/64];
    let mut i = 0;
    let mut j;
    loop {
        if i == H {
            break
        }
        j = 0;
        loop {
            if j == W {
                break
            }
            if (i*W+j) % 2 == 0 {
                let idx = (i*W+j)>>6;
                let offset = (i*W+j) % 64;
                arr[idx] |= 1_u64<<offset;
            }
            j += 1;
        }
        i += 1;
    }
    Bitset::<{W*H}>::from_array(arr)
}

/// Computes LEFT_EDGE_MASK and RIGHT_EDGE_MASK
pub const fn vertical_edge_mask<const W: usize, const H: usize>(right: bool) -> Bitset<{W*H}>
where [(); (W*H+63)/64]: Sized {
    let mut arr = [0_u64; (W*H+63)/64];
    let mut i = 0;
    let j = if right { W-1 } else { 0 };
    loop {
        if i == H {
            break
        }
        let idx = (i*W+j)>>6;
        let offset = (i*W+j) % 64;
        arr[idx] |= 1_u64<<offset;
        i += 1;
    }
    Bitset::<{W*H}>::from_array(arr)
}

/// Computes TOP_EDGE_MASK and BOTTOM_EDGE_MASK
pub const fn horizontal_edge_mask<const W: usize, const H: usize>(top: bool) -> Bitset<{W*H}>
where [(); (W*H+63)/64]: Sized {
    let mut arr = [0_u64; (W*H+63)/64];
    let i = if top { H-1 } else { 0 };
    let mut j = 0;
    loop {
        if j == W {
            break
        }
        let idx = (i*W+j)>>6;
        let offset = (i*W+j) % 64;
        arr[idx] |= 1_u64<<offset;
        j += 1;
    }
    Bitset::<{W*H}>::from_array(arr)
}

/// Computes possible moves from every position at compile time
pub const fn precompute_moves<const S: usize, const W: usize, const H: usize, const WRAP: bool>
() -> [[Option<u16>; 4]; W*H]
where [(); (W*H+63)/64]: Sized, [(); W*H]: Sized {
    let mut result = [[None; 4]; {W*H}];
    let mut pos = 0;
    loop {
        if pos == W*H {
            break
        }
        if WRAP {
            // up
            let move_to = (pos + W) % (W*H);
            result[pos][0] = Some(move_to as u16);
            
            // down
            let move_to = if W > pos { W*(H-1) + pos } else { pos - W };
            result[pos][1] = Some(move_to as u16);
            
            // right
            let move_to = if pos % W == W-1 { pos - (W-1) } else { pos + 1};
            result[pos][2] = Some(move_to as u16);
            
            // left
            let move_to = if pos % W == 0 { pos + (W-1) } else { pos - 1 };
            result[pos][3] = Some(move_to as u16);
        } else {
            // up
            if pos < W * (H-1) {
                let move_to = pos + W;
                result[pos][0] = Some(move_to as u16);
            }
            // down
            if pos >= W {
                let move_to = pos - W;
                result[pos][1] = Some(move_to as u16);
            }
            // right
            if pos % W < W - 1 {
                let move_to = pos + 1;
                result[pos][2] = Some(move_to as u16);
            }
            // left
            if pos % W > 0 {
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
