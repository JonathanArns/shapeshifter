use crate::types::*;

#[derive(Clone, Debug)]
pub struct Snake {
    pub is_enemy: bool,
    pub length: u8,
    pub health: u8,
    pub body: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct MailBoxBoard {
    pub board: Vec<Square>,
    pub width: usize,
    pub height: usize,
    pub snakes: Vec<Snake>,
}

impl MailBoxBoard {
    pub fn new(w: &usize, h: &usize) -> MailBoxBoard {
        MailBoxBoard{
            board: vec![0; w*h],
            width: *w,
            height: *h,
            snakes: Vec::new(),
        }
    }

    fn checked_move_dest(&self, mv: &Move, from: usize) -> Option<usize> {
        match mv {
            Move::Up => if from + self.width < self.board.len() {
                Some(from + self.width)
            } else {
                None
            },
            Move::Down => if from > self.width {
                Some(from - self.width)
            } else {
                None
            },
            Move::Left => if from % self.width > 0 {
                Some(from - 1)
            } else {
                None
            },
            Move::Right => if (from + 1) % self.width > from % self.width {
                Some(from + 1)
            } else {
                None
            },
        }
    }

    pub fn children(&self) -> [Option<(Move, Vec<MailBoxBoard>)>; 4] {
        let mut ret = [None, None, None, None];
        let mut enemy_moves = vec![vec![]];
        let mut my_moves = vec![];
        for (si, snake) in self.snakes.iter().enumerate() {
            let mut moves = Vec::new();
            let mut some_legal_move = Move::Up;
            for mv in [Move::Up, Move::Down, Move::Left, Move::Right] {
                if let Some(i) = self.checked_move_dest(&mv, snake.body[0]) {
                    some_legal_move = mv;
                    if self.board[i] & (HEAD | BODY) == 0 {
                        moves.push((si, mv))
                    }
                }
            }
            if snake.is_enemy {
                if moves.len() == 0 {
                    moves.push((si, some_legal_move))
                }
                enemy_moves = moves.iter().flat_map(move |mv| {
                    let mut tmp = enemy_moves.clone();
                    for ref mut mvs in &mut tmp {
                        mvs.push(mv.clone())
                    }
                    tmp
                }).collect();
            } else {
                my_moves = moves;
            }
        }
        for mv in my_moves {
            let mut boards: Vec<Self> = vec![];
            for mvs in &enemy_moves {
                let mut x = mvs.clone();
                x.push(mv);
                boards.push(standard(self, &x))
            }
            ret[mv.1 as usize] = Some((mv.1, boards));
        }
        return ret;
    }
    
    pub fn is_terminal(&self) -> bool {
        if self.snakes.len() < 2 {
            return true
        }
        for snake in &self.snakes {
            if !snake.is_enemy {
                return false
            }
        }
        return true
    }

    pub fn eval(&self) -> Score {
        let mut score: Score = 0;
        let n = self.snakes.len() as Score;
        if n == 1 {
            if self.snakes[0].is_enemy {
                return Score::MIN
            } else {
                return Score::MAX
            }
        }
        for snake in &self.snakes {
            if snake.is_enemy {
                score -= snake.length as Score;
            } else {
                score += snake.length as Score * n;
            }
        }
        if n == 0 {
            return score
        }
        score / n
    }

    pub fn get(&mut self, i: usize) -> &mut Square {
        &mut self.board[i]
    }
}

impl Board for MailBoxBoard {
    fn num_snakes(&self) -> usize {
        self.snakes.len()
    }

    fn alphabeta(&self, d: u8, mut alpha: Score, mut beta: Score) -> (Move, Score, u8) {
        let beta_init = beta;
        if d == 0 || self.is_terminal() {
            return (Move::Up, self.eval(), d)
        }
        let mut max = (Move::Up, Score::MIN, d);
        let my_moves = self.children();
        for maybe_mv in my_moves {
            if let Some((mv, positions)) = maybe_mv {
                beta = beta_init; // because the inner loop is essentially the minimizing call
                let mut min = Score::MAX;
                let mut min_depth = d;
                for position in positions {
                    let (_, score, depth) = position.alphabeta(d-1, alpha, beta);
                    if score < min {
                        min = score;
                        min_depth = depth;
                    }
                    if min < beta {
                        beta = min;
                        if beta < alpha {
                            break
                        }
                    }
                }
                if min > max.1 || (min == max.1 && min_depth < max.2) {
                    max = (mv, min, min_depth);
                    if max.1 > alpha {
                        alpha = max.1;
                        if beta < alpha {
                            break
                        }
                    }
                }
            }
        }
        max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_dist_checked() {
        let b = MailBoxBoard::new(&4, &4);
        // forbidden moves
        assert_eq!(b.checked_move_dest(&Move::Left, 4), None);
        assert_eq!(b.checked_move_dest(&Move::Right, 7), None);
        assert_eq!(b.checked_move_dest(&Move::Up, 14), None);
        assert_eq!(b.checked_move_dest(&Move::Down, 2), None);
        assert_eq!(b.checked_move_dest(&Move::Right, 3), None);
        assert_eq!(b.checked_move_dest(&Move::Down, 3), None);
        // allowed moves
        assert_ne!(b.checked_move_dest(&Move::Left, 7), None);
        assert_ne!(b.checked_move_dest(&Move::Right, 4), None);
        assert_ne!(b.checked_move_dest(&Move::Up, 3), None);
        assert_ne!(b.checked_move_dest(&Move::Down, 14), None);
    }
}

fn standard(b: &MailBoxBoard, moves: &Vec<(usize, Move)>) -> MailBoxBoard {
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

fn move_snakes(b: &mut MailBoxBoard, moves: &Vec<(usize, Move)>) {
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

fn remove_snake(b: &mut MailBoxBoard, si: usize) {
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
