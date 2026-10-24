# KeyPrompter

A cue strip for streaming. It shows "mini slides" from a Markdown file at the top of the screen, near the webcam. The current slide sits at the horizontal middle. Upcoming slides wait on the right, and past slides shrink into a trail on the left.

## Run

```sh
cargo run --release --offline -- path/to/rundown.md
```

Without an argument it reopens the last rundown. Use File > Open (⌘O) to pick another one. Click the strip once to give it keyboard focus. Drag it to move it. The position and the open rundown are saved in `~/Library/Application Support/KeyPrompter/state.txt`.

| Key | Action |
|---|---|
| → Space PageDown ↓ | Next slide (the first press starts the timer) |
| ← PageUp ↑ Backspace | Previous slide |
| Home | First slide, reset the timer |
| End | Last slide |
| ⌃⌥→ / ⌃⌥← | Next / previous slide from any app, no focus needed |
| ⌘O | Open a rundown |
| ⌘H | Hide |
| ⌘Q | Quit |

To build `target/release/KeyPrompter.app` (ad-hoc signed, opens `.md` files from Finder's Open With):

```sh
scripts/bundle.sh
```

The strip is hidden from screenshots, screen sharing and recordings by default. View > Hide from Screen Capture turns that off. With Reduce Motion on in System Settings, slides change without the slide animation.

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
