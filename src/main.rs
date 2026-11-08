use std::{
    io::{self, Write, stdout},
    time::{Duration, Instant},
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute, queue,
    style::Print,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::{SeedableRng, seq::SliceRandom};

const BOARD_WIDTH: usize = 40;
const BOARD_HEIGHT: usize = 40;
const CELL_WIDTH: usize = 2;
const MIN_TERMINAL_WIDTH: u16 = (BOARD_WIDTH * CELL_WIDTH + 3) as u16;
const MIN_TERMINAL_HEIGHT: u16 = (BOARD_HEIGHT + 5) as u16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Shape {
    const ALL: [Self; 7] = [
        Self::I,
        Self::O,
        Self::T,
        Self::S,
        Self::Z,
        Self::J,
        Self::L,
    ];

    fn cells(self) -> [(i32, i32); 4] {
        match self {
            Self::I => [(0, 1), (1, 1), (2, 1), (3, 1)],
            Self::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
            Self::T => [(1, 0), (0, 1), (1, 1), (2, 1)],
            Self::S => [(1, 0), (2, 0), (0, 1), (1, 1)],
            Self::Z => [(0, 0), (1, 0), (1, 1), (2, 1)],
            Self::J => [(0, 0), (0, 1), (1, 1), (2, 1)],
            Self::L => [(2, 0), (0, 1), (1, 1), (2, 1)],
        }
    }

    fn matrix(self) -> Vec<Vec<bool>> {
        let size = match self {
            Self::I => 4,
            Self::O => 2,
            _ => 3,
        };
        let mut matrix = vec![vec![false; size]; size];
        for (x, y) in self.cells() {
            matrix[y as usize][x as usize] = true;
        }
        matrix
    }
}

fn rotate_matrix(matrix: &[Vec<bool>]) -> Vec<Vec<bool>> {
    let size = matrix.len();
    (0..size)
        .map(|x| (0..size).rev().map(|y| matrix[y][x]).collect())
        .collect()
}

struct ShapeBag {
    rng: rand::rngs::StdRng,
    remaining: Vec<Shape>,
}

impl ShapeBag {
    fn new(seed: Option<u64>) -> Self {
        let rng = match seed {
            Some(seed) => rand::rngs::StdRng::seed_from_u64(seed),
            None => rand::rngs::StdRng::from_entropy(),
        };
        Self {
            rng,
            remaining: Vec::new(),
        }
    }

    fn next(&mut self) -> Shape {
        if self.remaining.is_empty() {
            self.remaining.extend(Shape::ALL);
            self.remaining.shuffle(&mut self.rng);
        }
        self.remaining.pop().expect("shape bag was refilled")
    }
}

#[derive(Clone, Copy, Debug)]
struct Piece {
    shape: Shape,
    x: i32,
    y: i32,
    rotation: u8,
}

impl Piece {
    fn new(shape: Shape) -> Self {
        Self {
            shape,
            x: (BOARD_WIDTH as i32 - 4) / 2,
            y: 0,
            rotation: 0,
        }
    }

    fn rotated(self) -> Self {
        Self {
            rotation: (self.rotation + 1) % 4,
            ..self
        }
    }

    fn matrix(self) -> Vec<Vec<bool>> {
        let mut matrix = self.shape.matrix();
        for _ in 0..self.rotation {
            matrix = rotate_matrix(&matrix);
        }
        matrix
    }

    fn cells(self) -> impl Iterator<Item = (i32, i32)> {
        let matrix = self.matrix();
        matrix.into_iter().enumerate().flat_map(move |(y, row)| {
            row.into_iter()
                .enumerate()
                .filter_map(move |(x, occupied)| {
                    occupied.then_some((self.x + x as i32, self.y + y as i32))
                })
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Board {
    cells: [[bool; BOARD_WIDTH]; BOARD_HEIGHT],
}

impl Board {
    fn new() -> Self {
        Self {
            cells: [[false; BOARD_WIDTH]; BOARD_HEIGHT],
        }
    }

    fn can_place(&self, piece: Piece) -> bool {
        piece.cells().all(|(x, y)| {
            x >= 0
                && x < BOARD_WIDTH as i32
                && y >= 0
                && y < BOARD_HEIGHT as i32
                && !self.cells[y as usize][x as usize]
        })
    }

    fn lock(&mut self, piece: Piece) {
        for (x, y) in piece.cells() {
            if (0..BOARD_WIDTH as i32).contains(&x) && (0..BOARD_HEIGHT as i32).contains(&y) {
                self.cells[y as usize][x as usize] = true;
            }
        }
    }

    fn clear_full_lines(&mut self) -> usize {
        let mut remaining = [[false; BOARD_WIDTH]; BOARD_HEIGHT];
        let mut destination = BOARD_HEIGHT;
        let mut cleared = 0;

        for source in (0..BOARD_HEIGHT).rev() {
            if self.cells[source].iter().all(|occupied| *occupied) {
                cleared += 1;
            } else {
                destination -= 1;
                remaining[destination] = self.cells[source];
            }
        }

        self.cells = remaining;
        cleared
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Input {
    Left,
    Right,
    Rotate,
    SoftDrop,
    Quit,
}

fn input_from_key(key: KeyEvent) -> Option<Input> {
    if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
        return None;
    }
    match key.code {
        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => Some(Input::Left),
        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => Some(Input::Right),
        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => Some(Input::Rotate),
        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => Some(Input::SoftDrop),
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => Some(Input::Quit),
        _ => None,
    }
}

struct Game {
    board: Board,
    active: Piece,
    bag: ShapeBag,
    lines_cleared: usize,
    game_over: bool,
}

impl Game {
    fn new(seed: Option<u64>) -> Self {
        let mut bag = ShapeBag::new(seed);
        let active = Piece::new(bag.next());
        let game_over = !Board::new().can_place(active);
        Self {
            board: Board::new(),
            active,
            bag,
            lines_cleared: 0,
            game_over,
        }
    }

    fn handle_input(&mut self, input: Input) {
        let candidate = match input {
            Input::Left => Piece {
                x: self.active.x - 1,
                ..self.active
            },
            Input::Right => Piece {
                x: self.active.x + 1,
                ..self.active
            },
            Input::Rotate => {
                let rotated = self.active.rotated();
                for offset in [0, -1, 1, -2, 2] {
                    let candidate = Piece {
                        x: rotated.x + offset,
                        ..rotated
                    };
                    if self.board.can_place(candidate) {
                        self.active = candidate;
                        return;
                    }
                }
                return;
            }
            Input::SoftDrop => Piece {
                y: self.active.y + 1,
                ..self.active
            },
            Input::Quit => return,
        };
        if self.board.can_place(candidate) {
            self.active = candidate;
        } else if input == Input::SoftDrop {
            self.lock_active();
        }
    }

    fn tick(&mut self) {
        if self.game_over {
            return;
        }
        let candidate = Piece {
            y: self.active.y + 1,
            ..self.active
        };
        if self.board.can_place(candidate) {
            self.active = candidate;
        } else {
            self.lock_active();
        }
    }

    fn lock_active(&mut self) {
        self.board.lock(self.active);
        self.lines_cleared += self.board.clear_full_lines();
        self.active = Piece::new(self.bag.next());
        self.game_over = self.board.cells[0].iter().any(|&occupied| occupied)
            || !self.board.can_place(self.active);
    }

    fn occupied(&self, x: usize, y: usize) -> bool {
        self.board.cells[y][x]
            || (!self.game_over
                && self
                    .active
                    .cells()
                    .any(|(piece_x, piece_y)| piece_x == x as i32 && piece_y == y as i32))
    }
}

fn terminal_is_large_enough() -> io::Result<bool> {
    let (width, height) = terminal::size()?;
    Ok(width >= MIN_TERMINAL_WIDTH && height >= MIN_TERMINAL_HEIGHT)
}

fn render(game: &Game) -> io::Result<()> {
    let mut out = stdout();
    queue!(out, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

    if !terminal_is_large_enough()? {
        queue!(
            out,
            Print(format!(
                "Terminal too small. Resize to at least {MIN_TERMINAL_WIDTH}x{MIN_TERMINAL_HEIGHT} cells.\r\n"
            )),
            Print("Press Q or Esc to quit.\r\n")
        )?;
        return out.flush();
    }

    let border = format!("+{}+\r\n", "-".repeat(BOARD_WIDTH * CELL_WIDTH));
    queue!(out, Print(&border))?;
    for y in 0..BOARD_HEIGHT {
        queue!(out, Print("|"))?;
        for x in 0..BOARD_WIDTH {
            let cell = if game.occupied(x, y) { "##" } else { "  " };
            queue!(out, Print(cell))?;
        }
        queue!(out, Print("|\r\n"))?;
    }
    queue!(
        out,
        Print(&border),
        Print("A/D or arrows: move   W/up: rotate   S/down: drop   Q/Esc: quit\r\n"),
        Print(format!("Lines cleared: {}\r\n", game.lines_cleared))
    )?;
    if game.game_over {
        queue!(out, Print("GAME OVER\r\n"))?;
    }
    out.flush()
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen, cursor::Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), cursor::Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn run() -> io::Result<Option<usize>> {
    let _terminal = TerminalGuard::enter()?;
    let mut game = Game::new(None);
    let mut last_tick = Instant::now();
    render(&game)?;

    loop {
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if let Some(input) = input_from_key(key) {
                    if input == Input::Quit {
                        break;
                    }
                    game.handle_input(input);
                    render(&game)?;
                    if game.game_over {
                        break;
                    }
                }
            }
        }
        if last_tick.elapsed() >= Duration::from_millis(600) {
            game.tick();
            last_tick = Instant::now();
            render(&game)?;
            if game.game_over {
                break;
            }
        }
    }
    Ok(game.game_over.then_some(game.lines_cleared))
}

fn main() -> io::Result<()> {
    if let Some(lines) = run()? {
        println!("Game over: the tower reached the top. Lines cleared: {lines}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shape_has_four_cells() {
        for shape in Shape::ALL {
            assert_eq!(shape.cells().len(), 4);
        }
    }

    #[test]
    fn rotation_matrix_matches_each_piece_longest_side() {
        assert_eq!(Piece::new(Shape::I).matrix().len(), 4);
        assert_eq!(Piece::new(Shape::O).matrix().len(), 2);
        assert_eq!(Piece::new(Shape::T).matrix().len(), 3);
    }

    #[test]
    fn piece_moves_inside_boundaries() {
        let mut game = Game::new(Some(1));
        for _ in 0..BOARD_WIDTH {
            game.handle_input(Input::Left);
        }
        assert!(game.active.cells().all(|(x, _)| x >= 0));
        for _ in 0..BOARD_WIDTH {
            game.handle_input(Input::Right);
        }
        assert!(game.active.cells().all(|(x, _)| x < BOARD_WIDTH as i32));
    }

    #[test]
    fn soft_drop_locks_at_floor() {
        let mut game = Game::new(Some(2));
        for _ in 0..BOARD_HEIGHT {
            game.handle_input(Input::SoftDrop);
        }
        assert!(game.board.cells.iter().flatten().any(|occupied| *occupied));
    }

    #[test]
    fn full_lines_are_removed_and_rows_above_fall() {
        let mut board = Board::new();
        board.cells[BOARD_HEIGHT - 1] = [true; BOARD_WIDTH];
        board.cells[BOARD_HEIGHT - 2][0] = true;

        assert_eq!(board.clear_full_lines(), 1);
        assert!(!board.cells[BOARD_HEIGHT - 1][BOARD_WIDTH - 1]);
        assert!(board.cells[BOARD_HEIGHT - 1][0]);
    }

    #[test]
    fn rotation_changes_non_square_piece_and_stays_in_bounds() {
        let mut game = Game::new(Some(3));
        game.active = Piece::new(Shape::T);
        game.handle_input(Input::Rotate);
        assert_eq!(game.active.rotation, 1);
        assert!(game.board.can_place(game.active));
    }

    #[test]
    fn i_piece_rotates_as_a_whole_from_horizontal_to_vertical() {
        let horizontal = Piece::new(Shape::I);
        let vertical = horizontal.rotated();

        assert_eq!(
            horizontal.cells().collect::<Vec<_>>(),
            vec![(18, 1), (19, 1), (20, 1), (21, 1)]
        );
        assert_eq!(
            vertical.cells().collect::<Vec<_>>(),
            vec![(20, 0), (20, 1), (20, 2), (20, 3)]
        );

        let full_turn = vertical.rotated().rotated().rotated();
        assert_eq!(
            full_turn.cells().collect::<Vec<_>>(),
            horizontal.cells().collect::<Vec<_>>()
        );
    }

    #[test]
    fn locked_cells_block_new_piece() {
        let mut board = Board::new();
        board.cells[0] = [true; BOARD_WIDTH];
        let piece = Piece::new(Shape::O);
        assert!(!board.can_place(piece));
    }

    #[test]
    fn locking_a_piece_at_the_top_ends_the_game() {
        let mut game = Game::new(Some(4));
        game.active = Piece {
            shape: Shape::O,
            x: 0,
            y: 0,
            rotation: 0,
        };
        game.board.cells[2][0] = true;
        game.tick();

        assert!(game.game_over);
        assert!(game.board.cells[0][0]);
    }

    #[test]
    fn input_aliases_are_supported() {
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('a'))),
            Some(Input::Left)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('D'))),
            Some(Input::Right)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('w'))),
            Some(Input::Rotate)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Down)),
            Some(Input::SoftDrop)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Esc)),
            Some(Input::Quit)
        );
    }
}
