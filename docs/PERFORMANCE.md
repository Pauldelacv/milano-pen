# Performance

## How the app stays fast

See [ARCHITECTURE.md §5–6](ARCHITECTURE.md#5-rendering-strategy) for the
design. In short: only cards in view (+50 % overscan) are mounted, found
through a grid spatial index; the camera transform is applied once per frame
outside React; moving a card re-renders only that card; images are shown as
256 px or 1024 px thumbnails chosen from on-screen size and pixel density,
generated in the background by a bounded worker pool, and the original is
loaded only in the viewer or when zoomed in far enough to need it.

## Benchmark board

**Help → Generate Benchmark Board** (or Settings → Developer) creates a board
with:

| | Count |
| --- | --- |
| Images (distinct 1280×960 JPEGs) | 300 |
| Notes (rich text) | 520 |
| Columns, each with 5 cards inside | 20 (+100) |
| Nested boards, each with 2 notes | 40 |
| Lines between cards | 60 |
| **Cards on the board** | **1,040** |

The whole generation is one undo step.

## Automated checks

- `cargo test -p milano-core --test workspace benchmark_board_is_fast_to_open`
  builds the same board (with small images) and asserts it opens in under 2 s
  in a debug build, and that undo removes it in one step.
- `src/canvas/canvas.test.ts` asserts 100 viewport queries over a 20,000-card
  spatial index run in well under 500 ms.

## Manual checklist (per release, on Apple Silicon and Intel)

1. Generate the benchmark board and open it.
2. Pan and pinch-zoom continuously for 30 s: motion should stay smooth (use
   Safari's Web Inspector → Timelines, enabled in dev builds, to check frames).
3. Zoom to fit (⌘1), then zoom into a corner: images appear as thumbnails
   first and sharpen without layout shifts.
4. Drag a 50-card selection around: no stutter; one save at the end.
5. Watch memory in Activity Monitor (Milano Pen + its WebContent process)
   while panning across the whole board; it should plateau, not grow.

## Measured so far

Only on Linux (WebKitGTK, software rendering, debug build, CI-class VM):
opening the 1,040-card board from the database takes well under 1 s; at 10 %
zoom all 880 top-level cards render; memory after visiting the whole board was
about 180 MB for the app process and 320 MB for the web process. **Numbers on
real Macs (Intel and Apple Silicon) are still to be collected** — please add
them here.

Known limit: when zoomed far out, every card in view is a full DOM card. A
level-of-detail mode (simplified cards below ~30 % zoom) is on the roadmap.
