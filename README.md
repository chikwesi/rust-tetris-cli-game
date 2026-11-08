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

## Origin and comparison

This is a Tetris game built in Rust for the command line, with ASCII graphics and keyboard controls. The initial prompt in this conversation was:

> We we building a tetris game in rust, it should run on the cli and use ascii characters for rendering. It would take keyboard inputs to move the pieces left and right, and the sssize of the board should be 400 * 400px in the terminal. include rotation

The first attempt used **GPT-5.6 Luna** in the separate [`tetris-cli`](../tetris-cli/) project. It did well at the start: that project has a playable terminal game with falling pieces, movement, line clearing, and the seven standard tetrominoes. The difficulty came during revisions. Adding and adjusting rotation took repeated effort, and I kept struggling to get the behavior I wanted. That friction was why I switched models. The current snapshot of the earlier project does contain rotation code; this account describes the development experience, not a claim that its final code lacks rotation.

This project was built with **[GPT-6 Sol](https://developers.openai.com/api/docs/models/gpt-6-sol)**. It reached the intended game in two prompts. Its remaining issue was smaller and more specific: the game kept running after the tower reached the top. A follow-up fixed the exit condition. The two projects also differ in board size: the earlier one uses a 10 × 20 playfield, while this one uses a 40 × 40 grid to represent the requested 400 × 400 logical area.

For this task, the better model produced the desired result with less back-and-forth. Better models can achieve better results faster, especially when a working first draft still needs careful changes.
