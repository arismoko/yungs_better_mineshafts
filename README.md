# YUNG's Better Mineshafts for Sunrise

Mineshafts redesigned: wide corridors, stairs and workstations, built from each biome's own wood and stone.

A port of [YUNG's Better Mineshafts](https://github.com/YUNG-GANG/YUNGs-Better-Mineshafts) by YUNGNICKYOUNG to [Sunrise](https://playsunrise.net) (Minecraft Java 26.3). The original builds its mineshafts in code, so this port's mineshaft generator is Rust, reading the original's thirteen structure definitions.

## What it does

- Thirteen mineshaft variants by biome (oak, spruce, snowy spruce, acacia, jungle, mesa, desert, red desert, ice, mushroom, lush, dripstone, overgrown), each with its own palette.
- A vertical entrance shaft with a ladder and doorway that opens onto the hillside through a surface tunnel (only where the ground drops away nearby), wide main corridors with supports, rails, lanterns, cobwebs and gravel piles, small tunnels with torches, turns, stairs and four- and five-way layered intersections, workstation rooms, ore deposits and zombie villager rooms, and the spider-cave dungeons below.
- The original's placement: its structure set, spacing and biome tags.
- Vanilla mineshafts no longer generate, as in the original (its "Disable Vanilla Mineshafts" option is on by default).
- The original's settings, applied when a world loads: the height range, every spawn rate, the outpost chance, the small shaft length, ore deposits and their eight ore chances.
- Operator commands `/bettermineshafts status` and `/bettermineshafts last [chunk_x chunk_z]` describe the generator and the last mineshaft it started.

## Differences from the original

- The four-way intersection keeps seven blocks of width where the original's box is a block off; the layout follows the original's chain otherwise.
- `/locate structure mineshaft` answers with the original's hint only when a player types it; through `execute`, a function, a command block or the console it still runs vanilla's search (sunrise-org/sunrise#5266).
- The status commands are this port's own; the original has none.

## Building

With the Sunrise mod tool (`cargo sunrise-mod`, from the Sunrise SDK) and Rust:

```sh
cargo sunrise-mod build    # compiles against the pinned game, packs target/sunrise-mod/bettermineshafts.smod
cargo sunrise-mod run      # builds it and starts the game with it
cargo test
```

`sunrise-mod.toml` pins the Sunrise Dev build `dev-2026-11-22-7cfdf81` (game commit `7cfdf81463`). Sunrise builds native mods from source on the player's computer, against the exact game build.

## Licence and credits

YUNG's Better Mineshafts is by YUNGNICKYOUNG (https://github.com/YUNG-GANG/YUNGs-Better-Mineshafts). Its structure definitions, palettes, tags and strings are theirs and are used here under the **GNU LGPL v3.0 only**, the original's licence; `LICENSE` holds the text and `LICENSE-NOTICE.md` says what this port carries.

Carried to Minecraft 26.3 by AdLf0208 ([AdLf0208/yungs_better_mineshafts](https://github.com/AdLf0208/yungs_better_mineshafts)), and ported to Sunrise by arismoko. This port, its Rust included, is distributed under the same licence.
