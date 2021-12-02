use crate::types::*;

pub type rules = fn (&Board, &Vec<(usize, Move)>) -> Board;

pub fn standard(b: &Board, moves: &Vec<(usize, Move)>) -> Board {
    let mut new = b.clone();
    
    // move snakes
    move_snakes(&mut new, moves);

    // reduce health
    for snake in &mut new.snakes {
        snake.health -= 1;

        // damage from hazards
        if new.board[snake.body[0]] & HAZARD != 0 {
            snake.health -= 15;
        }

    }

    // feed snakes
    let mut eaten = vec![];
    for snake in &mut new.snakes {
        if new.board[snake.body[0]] & FOOD != 0 {
            snake.health = 100;
            snake.length += 1;
            eaten.push(snake.body[0]);
        }
    }
    for i in eaten {
        new.board[i] &= u8::MAX ^ FOOD;
    }

//     for (i, field) in new.board.iter_mut().enumerate() {
// 
//         // feed snakes
//         if *field & (FOOD | HEAD) == (FOOD | HEAD) {
//             for snake in &mut new.snakes {
//                 if snake.body[0] == i {
//                     snake.health = 100;
//                     snake.length += 1;
//                 }
//             }
//             *field ^= FOOD;
//         }
//     }

    // eliminate snakes
    // starving first
    let mut starved = vec![];
    for (i, snake) in new.snakes.iter().enumerate() {
        if snake.health <= 0 {
            starved.push(i)
        }
    }
    starved.sort_unstable();
    let mut offset = 0;
    for i in starved {
        remove_snake(&mut new, i - offset);
        offset += 1;
    }

    // collisions 2nd
    let mut collided = vec![];
    for (i, snake) in new.snakes.iter().enumerate() {
        // body collisions
        if new.board[snake.body[0]] & (BODY | TAIL) != 0 {
            collided.push(i);
            continue // to avoid double removal in the edgecase of a head to head collision on a body
        }

        // head to head collisions
        for (j, s) in new.snakes.iter().enumerate() {
            if i != j && s.body[0] == snake.body[0] && snake.length <= s.length {
                collided.push(i);
                break
            }
        }
    }
    collided.sort_unstable();
    offset = 0;
    for i in collided {
        remove_snake(&mut new, i - offset);
        offset += 1;
    }

    new
}

fn move_snakes(b: &mut Board, moves: &Vec<(usize, Move)>) {
    for (i, snake) in b.snakes.iter_mut().enumerate() {
        // determine move for snake
        let mut mv = Move::Up;
        for (j, x) in moves {
            if i == *j {
                mv = *x;
                break
            }
        }
        // turn old head into body
        b.board[snake.body[0]] = BODY;

        // add new head
        let head = move_dest(&mv, snake.body[0], b.width);
        snake.body.insert(0, head);
        b.board[head] |= HEAD;

        // move tail, if necessary
        if snake.body.len() > snake.length as usize {
            b.board[*snake.body.last().unwrap()] ^= TAIL;
            snake.body.truncate(snake.length as usize);
            b.board[*snake.body.last().unwrap()] = TAIL;
        }
    }
}

fn move_dest(mv: &Move, from: usize, board_width: usize) -> usize {
    match mv {
        Move::Up => from + board_width,
        Move::Down => from - board_width,
        Move::Left => from - 1,
        Move::Right => from + 1,
    }
}

fn remove_snake(b: &mut Board, si: usize) {
    let snake = b.snakes.remove(si);
    let mut heads = vec![];
    for s in &b.snakes {
        heads.push(s.body[0]);
    }
    for i in snake.body {
        if !heads.contains(&i) {
            b.board[i] = 0;
        } else {
            b.board[i] = HEAD;
        }
    }
}
