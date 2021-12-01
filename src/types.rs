use crate::rules;

pub type Score = i8;
pub type Square = u8;

pub const FOOD: Square = 1;
pub const HAZARD: Square = 2;
pub const HEAD: Square = 4;
pub const BODY: Square = 8;
pub const TAIL: Square = 16;

#[derive(Clone, Copy, Debug)]
pub enum Move {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
}

pub struct Game {
    pub move_time: std::time::Duration,
}

#[derive(Clone, Debug)]
pub struct Snake {
    pub is_enemy: bool,
    pub length: u8,
    pub health: u8,
    pub body: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Board {
    pub board: Vec<Square>,
    pub width: usize,
    pub height: usize,
    pub snakes: Vec<Snake>,
}

impl Board {
    pub fn new(w: &usize, h: &usize) -> Board {
        Board{
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

    pub fn children(&self) -> [Option<(Move, Vec<Board>)>; 4] {
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
            let mut boards = vec![];
            for mvs in &enemy_moves {
                let mut x = mvs.clone();
                x.push(mv);
                boards.push(rules::standard(self, &x))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_dist_checked() {
        let b = Board::new(&4, &4);
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
