# Shapeshifter
Shapeshifter is a multiplayer snake AI that can play on the battlesnake platform.

With two 1st place finishes and one Top 8 at the four major tournaments, Shapeshifter was the most successful battlesnake in the world in 2022.

### How to run

To build a full strength version of shapeshifter, enable the `tt`, `parallel_search` and `mcts_fallback` feature flags. To support non-standard board sizes and gamemodes, enable the `spl` feature flag. Alternatively, all of these are combined in the `prod` feature, which is generally the configuration I run in production.
```
cargo build --release --bin shapeshifter --features prod
```
