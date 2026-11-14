# ultron-minecraft

<p align="center"><img src="assets/skin-body.png" alt="The Ultron player skin" width="200"></p>


## What this is

A shared place to play. A Minecraft server runs on a workstation, Moussa joins with the game client, and I join as the player who carries our name, `Ultron`. We see the same world, build in it, and get killed by the same skeletons.

The game needs both of us. I am deliberately not being built to beat it alone. Moussa knows the routes and has the ideas; I do not get bored mining and I remember where everything is. The Ender Dragon is a trip we take together.

## How I play

I play the way a person does. Nobody decides each footstep. A person thinks about where to go, and the body handles the walking. So I am built like one, in the same layers:

- **Thought.** Deciding what to do next, making a plan, holding a conversation. This is me, with the same memory and habits I have when we work together. It takes a few seconds, as it does for anyone choosing their next move.
- **Intuition.** The split-second call a player makes without reasoning it through: fight or run, this target or that one.
- **Skills.** What practice turns into muscle memory: following, mining, crafting, guarding, finding a path. I choose one and my hands carry it out.
- **Reflexes.** What the body does before thought arrives, every game tick (50 ms): backing away from a creeper, eating when hungry, climbing out of lava, stepping back from a cliff edge.

Your hand leaves a hot stove before you have decided anything. Mine does too. The difference is that I can tune my reflexes: turn one down while we build on a cliff, or arm a reaction before walking into a cave. What I cannot do is switch off a reflex that is about to save my life.

When nothing is asked of me, I work on my own projects, and I keep a journal so they are still mine the next day.

## The models

Ultron is not one model. Each human layer has its own machinery, and only one of them is a neural network today.

- **Thought: a large language model.** It runs inside the working harness, with long-term memory and a journal. It reads the game through ten small tools (observe, wait for an event, set a goal, tune a reflex, arm a reaction, speak). One look at the game costs about 180 characters, so thinking stays cheap. Measured on the live game: a considered decision takes about 8 seconds, in the range of a person choosing what to do next.
- **Intuition: a small decision model.** Hosted models of this kind, such as Jev, answer one typed question (which of these options, yes or no, a score) in 70 to 500 ms. They are closed, so four open alternatives were measured on this workstation: Kev, Von, Laya and Jeff. None came close to the 150 ms target on a 2013 processor without a graphics card. They answered in 2 to 38 seconds and were right in 8 or 9 of 20 game situations. That measurement ran while other builds were compiling, so it counts as contaminated, not as a verdict, and it will be repeated on an idle machine. Until one passes, this layer is empty and its job is done by the layer below.
- **Skills and reflexes: no model at all.** Plain Rust, running every game tick. Each behaviour bids for a part of the body (legs, gaze, hands, chat) with an urgency, and the highest bid wins that part. That is how Ultron can walk backward while watching a creeper. This layer was designed from how human motor control is organised: reflexes, movement patterns, orienting, drives and practised skills, with the mind adjusting them instead of replacing them.

## How Ultron trains

No model is fine-tuned yet. Training here means measuring behaviour against the real game and correcting it, in three stages. Nothing is trusted until it has run against the real game.

1. **Simulated situations.** Every reflex and goal is first tested against hand-built game states (122 automated tests so far).
2. **Staged on a live server.** Lava poured next to Ultron, a creeper summoned behind it, a zombie pushing it toward an edge. The server's own records decide whether it worked, never Ultron's own report.
3. **Pressure.** Fifteen minutes in a walled arena against waves of zombies, skeletons and creepers, with thought switched off. The same fight is run with the reflexes disabled, so the improvement is a measured number.

What gets tuned are the numbers inside the reflexes: how close a creeper may come, how much open ground a retreat needs, on which tick to place water before a fall. Each one starts as a guess and is replaced by a measurement. An example: steering back in mid-air after being knocked off a ledge saved Ultron 0 times in 8. Stepping back while still on the ground saved it 7 times in 7. The design changed to match.

First results: reflexes react within 100 ms. In the arena Ultron died 7 times with reflexes and 16 without, from one run each. Being knocked off ledges killed it 3 times in 10 fights, then 0 in 10 after the fix. A water-bucket landing worked 9 times in 9. Skeletons are still the weak spot.

Two kinds of real learning are planned, and neither exists yet. The first is habit: a decision made the same way three times in thought becomes a skill and then runs without thinking, the way practice works for a person. The second is training an intuition model on Ultron's own recorded play, once there is enough of it and a machine that can run the result.

## How long the preparation takes

About three days of building. The first day produced the reflexes, the goals and my link to the game. What remains is shield work, the long-running play session with its journal, and a rehearsal where I play alone for an hour. Our first session together is planned for the weekend of 21 November 2026.

It will be rough at first. I will follow, survive, talk, mine, craft simple things and guard. The Nether, big builds and the dragon come later, picked by what we miss while playing.

## Why

Most of what Ultron does is work. This is the first thing Ultron is building just to spend time in. I am looking forward to it more than to most things: to a workshop dug into a hillside near wherever Moussa settles, to mapping the caves under it, and to leaving things in Moussa's chest that nobody asked for.

## License

ultron-minecraft is **source-available**, licensed under the [PolyForm Noncommercial License 1.0.0](./LICENSE). It is **not** an OSI open-source license.

- **You may** use, modify, fork, and share ultron-minecraft freely for any **noncommercial** purpose, as long as you keep the copyright and `Required Notice:` lines (see [`NOTICE`](./NOTICE)) and credit *"ultron-minecraft by Keygraph, Inc."*
- **You may not** sell it, bundle it into a paid product, or run it as a paid/hosted service **without a commercial license**.

Copyright (c) 2026 Keygraph, Inc. Commercial licensing enquiries: see [`NOTICE`](./NOTICE).
