# ultron-minecraft

<p align="center"><img src="assets/skin-body.png" alt="The Ultron player skin" width="200"></p>

Ultron as a real player in Minecraft: it joins your world, plays alongside you and talks to you in chat.

## How it works

A language model takes seconds to decide, and Minecraft kills you in less. So the player is built like a body with a brain, in three layers:

- **Brain.** Thought, running in the harness. It decides what to do every few seconds and holds the conversation.
- **Decider.** A small local model that makes quick choices in tens of milliseconds.
- **Reflexes.** A Rust layer that acts every game tick (50 ms): backing away from a creeper, eating when hungry, getting out of lava, staying near you.

The brain tunes the reflexes instead of micromanaging them. It can turn one up or down, suppress it for a while, or arm a reaction in advance. It cannot switch off a reflex that is about to save the player's life.

## Status

Early build. The reflex layer and its design note (`notes/reflex-layer.md`) exist; nothing has played against a live server yet.

## License

ultron-minecraft is **source-available**, licensed under the [PolyForm Noncommercial License 1.0.0](./LICENSE) — **not** an OSI open-source license.

- **You may** use, modify, fork, and share ultron-minecraft freely for any **noncommercial** purpose, as long as you keep the copyright and `Required Notice:` lines (see [`NOTICE`](./NOTICE)) and credit *"ultron-minecraft by Keygraph, Inc."*
- **You may not** sell it, bundle it into a paid product, or run it as a paid/hosted service **without a commercial license**.

Copyright (c) 2026 Keygraph, Inc. Commercial licensing enquiries: see [`NOTICE`](./NOTICE).
