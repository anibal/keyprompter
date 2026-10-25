# KeyPrompter

A cue strip for streaming. It shows "mini slides" at the top of the screen, near the webcam. The current slide sits under the webcam, at the screen's center, as far as the strip reaches. Upcoming slides wait on the right, and past slides shrink into a trail on the left.

## Run

```sh
cargo run --release --offline -- path/to/rundown.md
```

KeyPrompter keeps its rundowns in a library at `~/Library/Application Support/KeyPrompter/Rundowns/`. A Markdown file passed on the command line, opened from Finder, or picked with File > Import Markdown (⌘O) is imported into the library the first time. Opening the same file again reopens the library copy, so later changes to the Markdown file are not picked up. File > Open Recent switches between rundowns, and File > Export Markdown writes the current one back out. The Card menu changes the card in the middle, and a right-click on any card changes that card where it is, leaving the middle card in place: Duplicate Card (the copy goes on the side facing the middle, and a copy of the middle card takes the middle), Merge with Next Card (its cue becomes a bullet), Move Card Before or After (past a section edge it joins the next section), and Delete Card. Every change is saved at once, and Edit > Undo (⌘Z) and Redo (⇧⌘Z) step through them, naming the change they revert. Cards glide to their new places after each change, lines a merge adds glow for a moment, and with Reduce Motion on they only fade. Click a card to bring it to the middle. To edit a card's text, double-click the card in the middle, press Return, or choose Card > Edit Card (⌘E). A right-click on any card offers Edit Card too, and brings that card to the middle first. The cue and the Markdown of the body become two fields on the card, in the same sizes, and a body taller than the card scrolls inside it. Return in the cue moves to the body, ↓ in the cue and ↑ on the body's first line cross between the fields, and Return in the body starts a new line. ⌘Return saves, and so does clicking elsewhere, switching to another app, moving to another slide, opening another rundown or changing the card. Esc discards. A save is one undo step named Edit Card, an unchanged card records none, and a cue left empty reverts. While a card is open, Edit > Undo and Redo step through the typing instead of the card changes. Without an argument it reopens the last rundown. Click the strip once to give it keyboard focus. Drag it to move it. Its edges are magnets. A screen edge, the screen's center line, the menu bar and the top of the screen pull the strip flush when it comes within 24 points, and a screen edge holds it until you push 48 points past. A strip flush left, centered or flush right is stuck there, and View > Stick to Left (⌥⌘←), Stick to Center (⌥⌘↓) and Stick to Right (⌥⌘→) put it back, at the width it last had on that side. Drag the thin zone on either outer edge to resize. An edge stuck to the left or right can't be grabbed. A centered strip, full width included, resizes from both edges at once, and so does any strip with Option held, which also unsticks it. Double-click an edge to push it to the screen edge, Option-double-click for full width. The strip is never narrower than 600 points. View > Move to Top of Screen puts it over the menu bar, closest to the webcam, and View > Move to Display does the same on another screen. Both keep the side. The side survives a resolution change, and if the saved screen is gone at launch, the strip opens on the main screen. The placement, the open rundown and the view settings are saved in `~/Library/Application Support/KeyPrompter/state.txt`.

| Key | Action |
|---|---|
| → Space PageDown ↓ | Next slide (the first press starts the timer) |
| ← PageUp ↑ Backspace | Previous slide |
| Home | First slide, reset the timer |
| End | Last slide |
| ⌃⌥→ / ⌃⌥← | Next / previous slide from any app, no focus needed |
| ⌥⌘← / ⌥⌘↓ / ⌥⌘→ | Stick the strip to the left, the center or the right of its screen |
| ⌥⌘↑ | Move the strip to the very top of its screen, over the menu bar |
| Return / ⌘E | Edit the card in the middle |
| ⌘Return | Save the edit |
| Esc | Discard the edit |
| ⌘O | Import a Markdown rundown |
| ⌘Z / ⇧⌘Z | Undo / redo a card change |
| ⌘H | Hide |
| ⌘Q | Quit |

To build `target/release/KeyPrompter.app` (ad-hoc signed, opens `.md` files from Finder's Open With):

```sh
scripts/bundle.sh
```

The icon in `assets/AppIcon.icns` comes from `swift scripts/icon.swift assets/AppIcon.icns`.

Text size follows the strip height: the cue line and four lines of text fill a card. View > Bigger Text, Smaller Text and Default Text Size adjust it, and the choice is saved. Every card is laid out once at full size, so the smaller cards beside the current one show the same lines, only smaller. Text never shrinks to fit: a card with more lines than it can hold gives its last row to a red "+N lines" badge, and the left column names it.

The strip is hidden from screenshots, screen sharing and recordings by default. View > Hide from Screen Capture turns that off. With Reduce Motion on in System Settings, slides change without the slide animation.

## Rundown format

```markdown
# Section name        (shown in the left column, not a slide)

## Slide cue          (the big line on the card)
- a bullet with **bold**, *italic* and `code`
A paragraph line.
```

Text before the first heading is ignored. Import reads this format and Export writes it.

## Check

```sh
cargo fmt --check && cargo clippy --offline --all-targets -- -D warnings && cargo test --offline
osascript -l JavaScript scripts/windows.js keyprompter   # on-screen windows, no Screen Recording permission needed
```

To try the bundled app without touching your own settings or library, `scripts/try.sh` runs a second copy with `KEYPROMPTER_HOME` set to `target/try-home`, which it wipes on each start. That copy also takes the actions the menus run from a `drive` pipe in that folder, one per line, with any data as JSON after the name.

```sh
scripts/try.sh start examples/stream.md
scripts/try.sh send Next 'EditCard "Duplicate"' Undo
scripts/try.sh shot strip.png    # fails while the strip is hidden from capture
scripts/try.sh stop
```
