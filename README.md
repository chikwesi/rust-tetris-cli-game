# ASCII Tetris

A terminal Tetris game in Rust. The board is a 40 × 40 cell grid representing a 400 × 400 logical pixel area (10 × 10 logical pixels per cell). Terminal character sizes vary, so it cannot guarantee a physical pixel size. Each board cell uses two ASCII columns to appear square in a typical terminal.

## Run

```sh
cargo run --release
```

Use a terminal at least 83 columns by 45 rows.

- Left/Right arrows or A/D: move
- Up arrow or W: rotate clockwise
- Down arrow or S: soft drop
- Q or Esc: quit

Filled cells use `##`; empty cells use spaces. Pieces fall automatically, lock when they land, and completed rows clear. The game quits automatically when the locked tower reaches the top row or a new piece cannot spawn.

## Project notes

This is a Tetris game built in Rust for the command line, with ASCII graphics and keyboard controls. The initial prompt in this conversation was:

> We we building a tetris game in rust, it should run on the cli and use ascii characters for rendering. It would take keyboard inputs to move the pieces left and right, and the sssize of the board should be 400 * 400px in the terminal. include rotation

We first tried building the game with GPT 5.6 Luna, and that attempt did not go as well. In this comparison, GPT 5.6 Soul produced a better result faster: it took two prompts, with one minor issue to fix—the game did not quit when the tower reached the top of the grid.

The takeaway from this experience is that better models can achieve better results faster.
