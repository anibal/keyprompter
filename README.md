# KeyPrompter

A cue strip for streaming. It shows "mini slides" from a Markdown file at the top of the screen, near the webcam. The current slide sits at the horizontal middle. Upcoming slides wait on the right, and past slides shrink into a trail on the left.

## Run

```sh
cargo run --release --offline -- path/to/rundown.md
```

Without an argument it opens `examples/stream.md`. Click the strip once to give it keyboard focus. Drag it to move it. The position is saved in `~/Library/Application Support/KeyPrompter/window.txt`.

| Key | Action |
|---|---|
| → Space PageDown ↓ | Next slide (the first press starts the timer) |
| ← PageUp ↑ Backspace | Previous slide |
| Home | First slide, reset the timer |
| End | Last slide |
| ⌘Q | Quit |

## Rundown format

```markdown
# Section name        (shown in the left column, not a slide)

## Slide cue          (the big line on the card)
- a bullet with **bold**, *italic* and `code`
A paragraph line.
```

Text before the first heading is ignored. Saving the file reloads it within a second and keeps your place.

## Check

```sh
cargo fmt --check && cargo clippy --offline --all-targets -- -D warnings && cargo test --offline
osascript -l JavaScript scripts/windows.js keyprompter   # on-screen windows, no Screen Recording permission needed
```
