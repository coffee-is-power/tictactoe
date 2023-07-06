use std::io::{stdin, stdout, Write};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use termion::color::{Fg, LightBlue, LightRed, Reset};
use termion::cursor::Goto;
use termion::event::{Event, Key, MouseButton, MouseEvent};
use termion::input::{MouseTerminal, TermRead};
use termion::raw::IntoRawMode;
type Cell = Option<Player>;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Player {
    X,
    O,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BoardState {
    Won(Player),
    Tie,
    Incomplete,
}
pub struct Board(pub [[Cell; 3]; 3]);
impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}
impl Board {
    pub fn new() -> Self {
        Self([[None; 3]; 3])
    }
    pub fn check_row(&self, row: usize) -> Option<Player> {
        (self.0[row][0] == self.0[row][1]
            && self.0[row][1] == self.0[row][2]
            && self.0[row][0].is_some())
        .then(|| self.0[row][0].unwrap())
    }

    pub fn check_col(&self, col: usize) -> Option<Player> {
        (self.0[0][col] == self.0[1][col]
            && self.0[1][col] == self.0[2][col]
            && self.0[0][col].is_some())
        .then(|| self.0[0][col].unwrap())
    }
    pub fn check_diagonal_top_left(&self) -> Option<Player> {
        (self.0[0][0] == self.0[1][1] && self.0[1][1] == self.0[2][2] && self.0[0][0].is_some())
            .then(|| self.0[0][0].unwrap())
    }

    pub fn check_diagonal_down_left(&self) -> Option<Player> {
        (self.0[2][0] == self.0[1][1] && self.0[1][1] == self.0[0][2] && self.0[2][0].is_some())
            .then(|| self.0[2][0].unwrap())
    }
    pub fn check_diagonals(&self) -> Option<Player> {
        self.check_diagonal_down_left()
            .or_else(|| self.check_diagonal_top_left())
    }
    pub fn has_empty_cells(&self) -> bool {
        self.0.iter().any(|col| col.iter().any(|c| c.is_none()))
    }
    pub fn state(&self) -> BoardState {
        for r in 0..3 {
            if let Some(team) = self.check_row(r) {
                return BoardState::Won(team);
            }
            if let Some(team) = self.check_col(r) {
                return BoardState::Won(team);
            }
        }
        if let Some(team) = self.check_diagonals() {
            BoardState::Won(team)
        } else if self.has_empty_cells() {
            BoardState::Incomplete
        } else {
            BoardState::Tie
        }
    }
    pub fn render_board(&self, x: u16, y: u16, stdout: &mut impl Write) -> std::io::Result<()> {
        for i in 0..3 {
            for j in 0..3 {
                let cell = self.0[i as usize][j as usize];
                write!(
                    stdout,
                    "{}{} --- ",
                    Fg(Reset),
                    Goto(x + (j * CELL_WIDTH), y + (i * CELL_HEIGHT))
                )?;
                write!(
                    stdout,
                    "{}| {}{} {}|",
                    Goto(x + (j * CELL_WIDTH), y + (i * CELL_HEIGHT) + 1),
                    match cell {
                        Some(Player::X) => Fg(LightBlue).to_string(),
                        Some(Player::O) => Fg(LightRed).to_string(),
                        None => Fg(Reset).to_string(),
                    },
                    match cell {
                        Some(Player::X) => 'X',
                        Some(Player::O) => 'O',
                        None => ' ',
                    },
                    Fg(Reset)
                )?;
                write!(
                    stdout,
                    "{}{} --- ",
                    Fg(Reset),
                    Goto(x + (j * CELL_WIDTH), y + (i * CELL_HEIGHT) + 2)
                )?;
            }
        }
        Ok(())
    }
}
pub struct Game {
    board: Board,
    current_player: Player,
}
impl Game {
    pub fn render(&self, stdout: &mut impl Write) {
        write!(stdout, "{}{}", termion::clear::All, termion::clear::All).unwrap();
        write!(
            stdout,
            "{}{}Tic Tac Toe",
            termion::clear::All,
            termion::cursor::Goto(1, 1)
        )
        .unwrap();
        self.board.render_board(BOARD_X, BOARD_Y, stdout).unwrap();
        match self.current_player {
            Player::X => write!(
                stdout,
                "{}Jogador Atual: {}X",
                Goto(BOARD_X, BOARD_Y + (CELL_HEIGHT * 3) + 3),
                termion::color::Fg(LightBlue),
            ),
            Player::O => write!(
                stdout,
                "{}Jogador Atual: {}O",
                Goto(BOARD_X, BOARD_Y + (CELL_HEIGHT * 3) + 3),
                termion::color::Fg(LightRed),
            ),
        }
        .unwrap();
        stdout.flush().unwrap();
    }
    pub fn switch_player(&mut self) {
        self.current_player = match self.current_player {
            Player::X => Player::O,
            Player::O => Player::X,
        };
    }
}
const BOARD_X: u16 = 10;
const BOARD_Y: u16 = 10;
const CELL_WIDTH: u16 = 5;
const CELL_HEIGHT: u16 = 3;
fn check_board_bounds(x: u16, y: u16) -> bool {
    (BOARD_Y..BOARD_Y + (CELL_HEIGHT * 3)).contains(&y)
        && (BOARD_X..BOARD_X + (CELL_WIDTH * 3)).contains(&x)
}
fn get_board_cell_position(screen_x: u16, screen_y: u16) -> (usize, usize) {
    let relative_x = screen_x - BOARD_X;
    let relative_y = screen_y - BOARD_Y;
    (
        (relative_x / CELL_WIDTH) as usize,
        (relative_y / CELL_HEIGHT) as usize,
    )
}
fn main() {
    let game = Arc::new(Mutex::new(Game {
        board: Board::default(),
        current_player: Player::X,
    }));
    let game2 = game.clone();
    let stdin = stdin();
    let mut stdout = MouseTerminal::from(stdout().into_raw_mode().unwrap());
    let (cs, cr) = mpsc::channel();
    std::thread::scope(|s| {
        s.spawn(move || {
            for c in stdin.events() {
                let evt = c.expect("Failed to read event");
                match evt {
                    Event::Mouse(MouseEvent::Press(MouseButton::Left, x, y)) => {
                        let mut game = game.lock().unwrap();
                        if !check_board_bounds(x, y) {
                            continue;
                        }
                        let (board_x, board_y) = get_board_cell_position(x, y);
                        let current_player = game.current_player;
                        let cell = &mut game.board.0[board_y][board_x];

                        if cell.is_some() {
                            continue;
                        }
                        *cell = Some(current_player);
                        game.switch_player();
                    }
                    Event::Key(Key::Char('q')) => {
                        cs.send(()).unwrap();
                        break;
                    }
                    _ => (),
                }
            }
        });
        s.spawn(move || {
            while cr.try_recv().is_err() {
                std::thread::sleep(Duration::from_millis(100));
                let game = game2.lock().unwrap();
                game.render(&mut stdout);
                match game.board.state() {
                    BoardState::Tie => {
                        write!(
                            stdout,
                            "{}{}EMPATE                                          ",
                            Fg(Reset),
                            Goto(BOARD_X, BOARD_Y + (CELL_HEIGHT * 3) + 3),
                        )
                        .unwrap();
                        std::process::exit(0);
                    }
                    BoardState::Incomplete => {}
                    BoardState::Won(player) => {
                        write!(
                            stdout,
                            "{}{}O jogador {}{player:#?}{} ganhou",
                            Goto(BOARD_X, BOARD_Y + (CELL_HEIGHT * 3) + 3),
                            Fg(Reset),
                            match player {
                                Player::X => Fg(LightBlue).to_string(),
                                Player::O => Fg(LightRed).to_string(),
                            },
                            Fg(Reset)
                        )
                        .unwrap();
                        std::process::exit(0);
                    }
                }
            }
        });
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn return_win_when_on_a_winning_board() {
        let board = Board([
            [Some(Player::X), Some(Player::X), Some(Player::X)],
            [None, None, None],
            [None, None, None],
        ]);
        assert_eq!(board.state(), BoardState::Won(Player::X));
        let board = Board([
            [None, None, None],
            [Some(Player::X), Some(Player::X), Some(Player::X)],
            [None, None, None],
        ]);
        assert_eq!(board.state(), BoardState::Won(Player::X));
        let board = Board([
            [None, None, None],
            [None, None, None],
            [Some(Player::X), Some(Player::X), Some(Player::X)],
        ]);
        assert_eq!(board.state(), BoardState::Won(Player::X));
        let board = Board([
            [Some(Player::X), None, None],
            [None, Some(Player::X), None],
            [None, None, Some(Player::X)],
        ]);
        assert_eq!(board.state(), BoardState::Won(Player::X));

        let board = Board([
            [None, None, Some(Player::X)],
            [None, Some(Player::X), None],
            [Some(Player::X), None, None],
        ]);
        assert_eq!(board.state(), BoardState::Won(Player::X));
    }

    #[test]
    fn return_incomplete_when_on_a_incomplete_board() {
        let board = Board([
            [Some(Player::X), None, None],
            [Some(Player::X), None, None],
            [None, None, None],
        ]);
        assert_eq!(board.state(), BoardState::Incomplete)
    }
}
