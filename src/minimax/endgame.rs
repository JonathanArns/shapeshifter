use crate::bitboard::*;
use bitssset::Bitset;

pub fn solver<
    const S: usize,
    const W: usize,
    const H: usize,
    const WRAP: bool,
    const HZSTACK: bool,
    const SILLY: u8,
>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK, SILLY>,
    my_area: &Bitset<{ W * H }>,
    enemy_area: &Bitset<{ W * H }>,
    my_area_size: i16,
    enemy_area_size: i16,
    my_food_distance: i16,
) -> Option<i16>
where
    [(); (W * H + 63) / 64]: Sized,
    [(); hz_stack_len::<HZSTACK, W, H>()]: Sized,
{
    if let Some(i) = _solver(board, my_area, enemy_area, my_area_size, enemy_area_size, my_food_distance) {
        if i != 0 {
            return Some(i16::MAX - board.turn as i16 - enemy_area_size)
        } else {
            return Some(i16::MIN + board.turn as i16 + my_area_size)
        }
    }
    None
}

// Returns the snake_index of the loser, if one can be determined
fn _solver<
    const S: usize,
    const W: usize,
    const H: usize,
    const WRAP: bool,
    const HZSTACK: bool,
    const SILLY: u8,
>(
    board: &Bitboard<S, W, H, WRAP, HZSTACK, SILLY>,
    my_area: &Bitset<{ W * H }>,
    enemy_area: &Bitset<{ W * H }>,
    my_area_size: i16,
    enemy_area_size: i16,
    my_food_distance: i16,
) -> Option<usize>
where
    [(); (W * H + 63) / 64]: Sized,
    [(); hz_stack_len::<HZSTACK, W, H>()]: Sized,
{
    if S != 2 || board.turn < 50 {
        return None
    }

    // make fill one bigger
    let my_area = my_area.clone();
    let enemy_area = enemy_area.clone();
    let mut em_fill = my_area | (Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::ALL_BUT_LEFT_EDGE_MASK & my_area)<<1 | (Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::ALL_BUT_RIGHT_EDGE_MASK & my_area)>>1 | my_area<<W | my_area>>W;
    let mut ee_fill = enemy_area | (Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::ALL_BUT_LEFT_EDGE_MASK & enemy_area)<<1 | (Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::ALL_BUT_RIGHT_EDGE_MASK & enemy_area)>>1 | enemy_area<<W | enemy_area>>W;
    if WRAP {
        em_fill |= (Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::LEFT_EDGE_MASK & my_area) >> (W-1)
            | (Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::RIGHT_EDGE_MASK & my_area) << (W-1)
            | (Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::BOTTOM_EDGE_MASK & my_area) << ((H-1)*W)
            | (Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::TOP_EDGE_MASK & my_area) >> ((H-1)*W);
        ee_fill |= (Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::LEFT_EDGE_MASK & enemy_area) >> (W-1)
            | (Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::RIGHT_EDGE_MASK & enemy_area) << (W-1)
            | (Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::BOTTOM_EDGE_MASK & enemy_area) << ((H-1)*W)
            | (Bitboard::<S, W, H, WRAP, HZSTACK, SILLY>::TOP_EDGE_MASK & enemy_area) >> ((H-1)*W);
    }
    
    // determine, when a tail comes by to chase
    let mut snake = board.snakes[0];
    let mut counter = 1;
    let mut my_tail_dist = 0;
    let mut enemey_tail_dist = 0;
    if em_fill.get(snake.tail as usize) && !(board.snakes[1].length > board.snakes[0].length && enemy_area.get(snake.tail as usize)) {
        my_tail_dist = counter;
    }
    if ee_fill.get(snake.tail as usize) && !(board.snakes[0].length > board.snakes[1].length && my_area.get(snake.tail as usize)) {
        enemey_tail_dist = counter;
    }
    let mut tail_pos = board.next_body_segment(snake.tail);
    while (my_tail_dist == 0 || enemey_tail_dist == 0) && tail_pos != snake.head {
        if my_tail_dist == 0 && em_fill.get(tail_pos as usize) {
            my_tail_dist = counter;
        }
        if enemey_tail_dist == 0 && ee_fill.get(tail_pos as usize) {
            enemey_tail_dist = counter;
        }
        counter += 1;
        tail_pos = board.next_body_segment(tail_pos);
    }
    if my_tail_dist == 0 {
        my_tail_dist = counter;
    }
    if enemey_tail_dist == 0 {
        enemey_tail_dist = counter;
    }
    snake = board.snakes[1];
    counter = 1;
    let mut my_etail_dist = 0;
    let mut enemey_etail_dist = 0;
    if em_fill.get(snake.tail as usize) && !(board.snakes[1].length > board.snakes[0].length && enemy_area.get(snake.tail as usize)) {
        my_etail_dist = counter;
    }
    if ee_fill.get(snake.tail as usize) && !(board.snakes[0].length > board.snakes[1].length && my_area.get(snake.tail as usize)) {
        enemey_etail_dist = counter;
    }
    tail_pos = board.next_body_segment(snake.tail);
    while (counter < my_tail_dist || counter < enemey_tail_dist) && (my_etail_dist == 0 || enemey_etail_dist == 0) && tail_pos != snake.head {
        if my_etail_dist == 0 && em_fill.get(tail_pos as usize) {
            my_etail_dist = counter;
        }
        if enemey_etail_dist == 0 && ee_fill.get(tail_pos as usize) {
            enemey_etail_dist = counter;
        }
        counter += 1;
        tail_pos = board.next_body_segment(tail_pos);
    }
    if my_etail_dist == 0 {
        my_etail_dist = counter;
    }
    if enemey_etail_dist == 0 {
        enemey_etail_dist = counter;
    }
    my_tail_dist = my_tail_dist.min(my_etail_dist);
    enemey_tail_dist = enemey_tail_dist.min(enemey_etail_dist);

    // decide loser or none
    if my_area_size < my_tail_dist && board.snakes[1].health as i16 > my_area_size {
        if enemy_area_size < enemey_tail_dist && board.snakes[0].health as i16 > enemy_area_size {
            if my_area_size < enemy_area_size {
                Some(0)
            } else if enemy_area_size < my_area_size {
                Some(1)
            } else {
                None
            }
        } else {
            Some(0)
        }
    } else if enemy_area_size < enemey_tail_dist && board.snakes[0].health as i16 > enemy_area_size {
        Some(1)
    } else if (board.snakes[0].health as i16) < my_food_distance {
        Some(0)
    } else {
        None
    }
}
