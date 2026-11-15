# Reflex layer: design note

Date: 2026-11-14. Status: design, no code yet. Scope: the fast local executor that runs at
20 ticks/s under Ultron's thought layer. Library-agnostic: every behaviour is a trigger over game
state and a response expressed as player inputs (movement keys, jump, sneak, sprint, yaw and
pitch, use, attack, hotbar slot, swap hands, inventory click, chat) or as a goal handed to a
path planner. Only legal player inputs; no packet tricks of the kind hack clients use.

One tick is 50 ms. "Budget" below means the maximum number of ticks between the trigger
becoming true and the first motor output.

## 0. The answer in one paragraph

The brief ("my reflexes react, not only my brain") is right, but it understates the gap. A
human's deliberate cortex is backed by a fast voluntary loop of 150 to 250 ms. Ultron's
thought loop is seconds (estimated 3 to 20 s per decision inside the harness, roughly 60 to
400 ticks; not measured yet). The executor must cover everything a human does in under about
one second: spinal and brainstem reflexes, defensive reactions, orienting, locomotor rhythm,
homeostatic upkeep, and the trained tactical skills (combat timing, shield timing, bridging)
that a human runs on fast cortical and cerebellar loops. The design below catalogs about 60
such behaviours, arbitrates them with a basal-ganglia-style selector (graded salience, per-body
channel, hysteresis, hard floors), gives the brain seven knobs (goal, mode, gains,
scoped inhibitions, armed if-then responses, attention, report budget), and sends the brain
only prediction errors through a habituating, token-budgeted upward channel. The part no
incumbent has is the physiology: graded and expiring top-down modulation with
imminence-bounded vetoes, brain-armed conditional reflexes, habituating upward reports, and
habits compiled from the brain's own repeated decisions.

## 1. The phenomenon: the human motor hierarchy

### 1.1 Levels, what they do, how fast

| Level | What it does | Human latency | Ticks | Design lesson |
|---|---|---|---|---|
| Spinal stretch reflex (short latency) | Ia afferent to motoneuron; resists unexpected limb displacement | about 20 to 50 ms [P14, K15] | under 1 | Fixed local loops need no world model; fire every tick |
| Long-latency (transcortical) stretch response | Same stimulus, but routed through motor cortex; scales with task goals and instruction | about 50 to 100 ms [P12, K15, P14] | 1 to 2 | Even a "reflex" can be goal-dependent if its gain is set from above |
| Vestibulo-ocular reflex | Counter-rotates eyes against head rotation | 7 to 15 ms; 8.6 ms mean in [CS00] | under 1 | Gaze stabilisation is separate from gaze choice |
| Acoustic startle | Brainstem burst: blink, neck (sternocleidomastoid first), then limbs, in caudal order [B91] | blink earliest; exact ms not read in this session (unverified) | 1 to 2 | A generic "something happened" interrupt that primes defence |
| StartReact | A loud stimulus releases an already prepared movement subcortically | premotor RT under 70 to 80 ms vs about 150 ms voluntary [VS99, C11] | 1 to 2 | The brain can pre-load a response that a fast trigger then fires |
| Automatic postural responses | Balance correction after support perturbation | about 70 to 120 ms (Horak and Macpherson, cited via review; primary not read) | 2 | Larger under postural threat (standing 160 cm vs 60 cm high, Carpenter et al. 2004, via secondary index) |
| Nociceptive withdrawal | Flexion away from a painful stimulus, modulated by task | literature commonly gives 90 to 130 ms for RIII; not verified this session | 2 | Contact with harm gets a fixed escape, gated by context |
| Defensive cascade (PAG, amygdala) | Freeze, flee, fight chosen by threat distance ("predatory imminence") [FL88]; as threat nears, control shifts from prefrontal cortex to periaqueductal grey [M07] | seconds to sub-second | n/a | The closer the threat, the less the planner may veto the reflex |
| Central pattern generators | Spinal networks produce the locomotor rhythm; brainstem locomotor region sets speed by a scalar drive [G85, D98] | rhythm, not latency | n/a | The goal sets speed and direction; the rhythm (sprint-jump, swim strokes) is local |
| Superior colliculus | Orients eyes and head to salient stimuli; express saccades | 80 to 120 ms express, about 150 to 200 ms regular [FB83, KLZ13] | 2 to 4 | Orienting is the attention gate that decides what reaches higher levels |
| Looming detection (SC) | Expanding image triggers escape or freeze in mice [YM13]; time-to-contact tau = size / rate of expansion [L76] | sub-second | 1 to 10 | Use time-to-contact, not distance, to rank approaching things |
| Cerebellum | Forward models predict sensory consequences of motor commands; timing; error correction [WMK98] | continuous | every tick | Predict the next state from your own inputs; act on the mismatch |
| Efference copy / corollary discharge | Predicted self-caused input is cancelled (you cannot tickle yourself) [BWF98] | continuous | every tick | Report only what was not caused or predicted by the agent itself |
| Basal ganglia: selection | Tonic inhibition of all motor programs; the most salient channel is released by disinhibition [Mi96, RPG99] | tens of ms | every tick | Central winner-take-all per resource, with hysteresis |
| Basal ganglia: habits and chunks | Repeated sequences are recoded as single units, with start and end "task bracketing" activity [Gr98, J99, Ma18]; habit vs goal-directed control arbitrated by relative uncertainty [DND05] | after training | n/a | Compile repeated brain decisions into triggered chunks; hand back when they become unreliable |
| Hypothalamic homeostasis | Drives proportional to deviation from set points; the value of food depends on internal state (alliesthesia) [Ca71]; homeostatic RL formalises this [KG14]; interoception surfaces body state to awareness [Cr02] | seconds to hours | 20 to 1000 | Drives have urgency curves, and most of them should surface upward as feelings, not act |
| Attachment | Proximity to a caregiver is regulated like a set point (Bowlby 1969, book) | seconds | 20 to 100 | "Stay near the human" is a homeostat with a set distance |
| Cortex, deliberate | Goal choice, planning, inhibition of prepotent responses | stop-signal RT about 200 to 250 ms [VL08]; deliberation seconds | 4+ | The LLM sits here, but about 100 times slower than human voluntary control |

### 1.2 How the cortex modulates the lower levels

These are the mechanisms the incumbents lack. Each maps to a knob or rule in section 4.

- **Set by instruction**: the long-latency stretch response changes with what the subject
  intends (Hammond 1956, J Physiol 132; primary not read, the claim is reported in [P12]).
  Design: reflex gains are parameters the brain sets per mode.
- **Gain by context**: postural responses grow when standing high (Carpenter et al. 2004, via
  secondary index). Design: some gains rise automatically from local context (near a drop,
  caution rises) as well as from the brain.
- **Prepared responses**: StartReact shows the cortex can store a ready response that a
  subcortical trigger releases at reflex latency [VS99, C11]. Implementation intentions ("if
  X then I will Y") make goal pursuit automatic in humans [Go99]. Design: the brain arms
  if-then responses that fire at tick latency.
- **Inhibition with limits**: humans can suppress a prepotent action (stop-signal paradigm
  [VL08]), but as threat imminence grows, control moves from prefrontal cortex to the
  periaqueductal grey [M07]. Design: brain inhibitions are honoured until predicted harm
  crosses a floor, after which the reflex fires anyway and the override is reported.
- **Prepulse inhibition and expectation**: a weak warning stimulus shortly before a startling
  one reduces the startle [Gm75]; self-produced input is attenuated by efference copy
  [BWF98]. Design: predicted or self-caused events are not reported upward.
- **Habituation and sensitization**: repeated harmless stimuli lose their response; a noxious
  event raises responsiveness for a while [TS66, GT70]. The orienting reflex fires on
  mismatch with the internal model and habituates to repetition [So63]; recently attended
  locations are deprioritised (inhibition of return [Kl00]). Design: the upward channel and
  the orienting reflex habituate; damage sensitises defensive gains.
- **Habit vs goal-directed arbitration**: the more reliable controller wins [DND05]; habits
  misfire when context resembles an old routine ("capture" slips [N81]). Design: habits are
  gated by the active goal and handed back to the brain when they fail.

### 1.3 The timescale mismatch (the lesson that changes the design)

In a human, combat footwork, shield timing and "jump at the edge" are learned voluntary or
cerebellar skills running at 150 to 500 ms. An LLM brain cannot run them at all. So the
executor holds two bands that a human keeps in different places: true reflexes (1 to 2 ticks)
and a tactical band (2 to 20 ticks) that in humans is fast cortex plus cerebellum. The brain
stays at the strategic band (what to do next, who to help, when to retreat for the night).

## 2. The exceptional case: what expert players automate

Sources: community practice and the game mechanics on minecraft.wiki. No peer-reviewed study
of Minecraft expertise was found; treat the novice/expert contrast as observation, not
measurement.

- **Water-bucket clutch (MLG)**: the expert selects the bucket and looks down while still
  falling, then uses it in the last ticks before impact and picks the water back up at once.
  The novice decides to do it too late. Lesson: anticipation from a forward model (time to
  landing), pre-positioning (slot and pitch) before the trigger, and a two-step chunk
  (place, retrieve). Water negates all fall damage; slime blocks, cobwebs, powder snow,
  ladders, vines and sweet berry bushes also negate it; hay bales and honey blocks reduce it
  to 20 percent [W-Dmg].
- **Block clutch**: when knocked off a bridge, the expert looks down and places a block under
  their feet mid-air. Lesson: an unexpected velocity (efference-copy mismatch) near a drop
  is a trigger in its own right.
- **Sneak at edges and speed bridging**: sneaking stops the player walking off an edge;
  bridging is a rhythmic chunk (step back, place, repeat). Lesson: a CPG-like rhythm with a
  safety reflex built in.
- **Sprint-jump travel**: sprint-jumping averages 7.127 m/s vs 5.612 m/s sprinting [W-Spr],
  at 0.2 exhaustion per jump vs 0.1 per metre sprinting [W-Hun]. Lesson: gait selection is a
  speed-vs-hunger trade the local layer can make.
- **Melee timing**: the expert waits for the attack cooldown instead of spamming, lands
  critical hits by striking while falling and not sprinting, uses sprint hits for knockback
  and releases forward to reset sprint (w-tap), and strafes to make the opponent miss.
  Critical-hit conditions and the sprint-knockback rule are from a wiki mirror and a mod
  page, not the primary wiki page (partially verified). Lesson: a cerebellar timing skill.
- **Shield timing**: a raised shield only blocks after 5 ticks [W-Sh]; experts raise it on
  the wind-up (a skeleton drawing its bow), not when the arrow appears. An axe hit disables
  the shield for 5 s [W-Sh]. Lesson: trigger on the cue that precedes the threat.
- **Totem discipline**: the totem works only in a hand [W-Tot]; experts keep it in the offhand
  in danger and re-equip a second one immediately after it pops. It does not save from the
  void [W-Tot].
- **Mining hygiene**: never dig straight down, never dig upward into gravel, block lava faces
  the moment they appear, carry a water bucket, light tunnels, block cave openings behind
  you. Novices die to exactly these. Lesson: pre-action vetoes (the dig is refused before
  it happens), which no incumbent reflex models.
- **Mob-specific stances**: walk away from creepers (fuse 30 ticks after coming within 3
  blocks; cancelled at 7 blocks with line of sight [W-Cr]); break line of sight with
  skeletons (shoot within 15 blocks, every 3 s on Normal, strafe [W-Sk]); never look at an
  enderman's head (looking for 5 ticks within 64 blocks provokes it [W-End]); keep looking
  at a creaking (it cannot move while a survival player looks within 60 degrees of it
  [W-Crk]); sneak away from a warden. Lesson: the same "hostile within N" trigger needs
  opposite responses by type, including the opposite gaze rule.
- **Pillar up when swarmed**: two or three blocks of height stop melee mobs. Lesson: a short
  escape chunk.
- **Topping off food**: health regenerates only at food 18 or above, and sprinting needs food
  above 6 [W-Hun]. Experts eat before fights and never let food fall to the point where they
  cannot run. Lesson: a homeostat whose set point depends on a second variable (health).
- **Fixed hotbar layout**: sword, pickaxe, blocks, food and water bucket always in the same
  slots. Lesson: a body schema. Reflexes are only fast if the tool is where the reflex
  expects it, so the layout is itself a homeostat.
- **Vigilance**: glancing behind in caves, listening for the creeper hiss and the bow string.
  Lesson: the orienting system includes periodic scanning and sound-driven turns.

## 3. Incumbent audit: fidelity to the biology

Legend: Y implemented, P partial, N absent. Based on the sources cited; items marked (u) were
not read in source for this note.

| System | Reflex / protective | Defensive by mob type | Locomotion rhythm | Orienting | Homeostatic | Habit / skill | Top-down modulation | Upward signalling | Forward model / anticipation |
|---|---|---|---|---|---|---|---|---|---|
| mineflayer-pathfinder [MF-PF] | N during execution; plan-time limits only (maxDropDown 4, dontMineUnderFallingBlock, dontCreateFlow) | N (entitiesToAvoid, entityCost are plan costs) | P (movement primitives, parkour, sprint) | N | N | N | P (static Movements config) | P (goal_reached, path_update, path_reset reasons) | P (planning, no runtime prediction) |
| mineflayer-pvp [MF-PVP] | N | N (attack a given target) | N | N | N | P (meleeAttackRate, attackRange) | P (fields set by code) | P (startedAttacking event) | N |
| mineflayer-auto-eat v5 [MF-AE] | N | N | N | N | Y hunger (minHunger 15, minHealth 14, banned foods) | N | P (options set by code) | P (eatStart, eatFinish, eatFail) | N |
| mineflayer-armor-manager [MF-AM] | N | N | N | N | Y equipment (equip better armor) | N | N | N | N |
| Baritone [BAR] | P plan-time: water-bucket fall as a planned move (allowWaterBucketFall, "Reliability: questionable", maxFallHeightBucket 20) | N (mob avoidance is a path cost, off by default) | Y (pathing, parkour, sprint) | N | N (no eat or combat settings) | P (mining, building processes) | P (static settings) | N (u) | P (plans falls) |
| Voyager [V23] | N (none described) | N | via Mineflayer | N | N (health only in the prompt) | P skill library of LLM-written code, retrieved by embedding and invoked by the LLM | N | P (chat feedback after code runs) | N |
| Mindcraft modes [MC-modes] | Y self_preservation (water, falling blocks, lava/fire, low health) | P (one generic "enemy": cowardice at 16 blocks, self_defense at 8) | P (unstuck after 20 s) | P (idle_staring is cosmetic, only when idle) | P (item_collecting, torch_placing) | N | P (setMode on/off only; numeric thresholds not exposed) | P (behaviour log flushed as an AUTO MESSAGE when a mode interrupted an action) | N (low-health response fires after damage) |
| Project Sid PIANO [PS24] | P ("reflex modules use small, fast non-LLM neural networks", not specified) | N | N (u) | N | N (authors: agents "lack robust innate drives") | N | N (controller broadcasts decisions; no gains described) | P (bottleneck into the cognitive controller) | N |
| Meteor client modules [MET] | Y (NoFall incl. bucket Place mode, AutoTotem) but some modes are protocol cheats | P (KillAura (u)) | N | N | P (AutoEat (u)) | N | P (human toggles and thresholds) | N | P (AutoTotem "Smart" predicts fall and explosion damage) |
| Parallel planning-acting [L25] | N (u, abstract only) | N (u) | via skill library | N | N | P skill library | P (planner issues interrupts) | P (shared memory) | N |
| FSM/BT bots, e.g. mineflayer-statemachine [MF-SM] | depends on author | depends | depends | N | depends | P (states as chunks) | N (fixed transitions) | N | N |

Per-system notes worth keeping:

- **Mindcraft is the closest incumbent** and must be the primary baseline. Its modes are
  ordered (self_preservation, unstuck, cowardice, self_defense, hunting, item_collecting,
  torch_placing, elbow_room, idle_staring); the loop stops at the first active mode. In the
  code defaults, cowardice (16 blocks) is checked before self_defense (8 blocks), so with
  both on the bot flees any hostile with a clear path before it can fight. Profiles may
  override the defaults (unverified). The LLM can only switch a mode on or off; distances,
  cooldowns and health thresholds are fields it cannot reach. idle_staring explicitly skips
  endermen, which is a hard-coded orienting inhibition, the right instinct, but not
  generalised.
- **Baritone and pathfinder implement the cerebellum's planning side** (good forward models of
  movement) but run them only at plan time. Nothing watches the world while executing,
  which is why falling blocks, mobs and damage break their plans.
- **Voyager's skill library is the closest thing to habits**, but its skills are retrieved and
  invoked by the LLM. A biological habit is triggered by the stimulus, without the cortex.
- **Meteor shows what a complete reflex set looks like** in a human-driven client, including
  anticipatory damage prediction, but it has no planner and its modulation is a human
  flipping toggles.

**Shared omission, stated explicitly.** Every incumbent implements some of the anatomy
(levels, modules, priorities) and none implements the physiology that connects the levels:

1. **No graded top-down modulation.** Reflexes are on/off (Mindcraft, Meteor) or statically
   configured (pathfinder, Baritone, auto-eat). No system lets the planner set gains, scope
   an inhibition to a place or a time, or have that inhibition expire.
2. **No limit on the planner's veto by threat imminence.** Where the planner can disable a
   reflex, it disables it completely, including in the situation the reflex exists for.
3. **No prepared responses.** No planner can arm a conditional response that fires at tick
   speed.
4. **No habit formation from the planner's own repeated decisions.** Voyager stores skills
   but keeps the LLM in the trigger loop.
5. **No habituating upward channel.** Signalling is either absent or a raw log flush. Nothing
   filters reports by novelty or prediction error, and nothing budgets tokens.
6. **Orienting is absent or cosmetic.** No incumbent uses gaze as the attention gate that
   decides what gets classified and reported.
7. **Reactions are mostly post-hoc.** Mindcraft reacts after damage; the exceptions are
   Meteor's AutoTotem and Baritone's planned bucket fall.

Items 1, 2, 5 and the habit part of 4 form the gap this design targets.

## 4. Design

### 4.1 Sensing and prediction (what the triggers read)

State available to any client library: hp, absorption, food, saturation, air (0 to 300
ticks), on-fire flag, active effects, position, velocity, on-ground, own fall distance,
inventory and held items with durability, blocks around the player, block light, time of
day, weather, entities (type, position, velocity, metadata such as the creeper fuse state or
"using item"), sound events with positions, damage events with source type and attacker,
chat, other players' look direction.

Derived every tick by a small forward model (the cerebellum):

- **t_land, dmg_land**: simulate own vertical motion (gravity 0.08 blocks/tick^2 [W-Attr];
  vertical drag 0.98 per tick, standard Java value, not re-read) against the block column to
  get ticks to landing and predicted fall damage, floor(fall distance - 3) adjusted for the
  landing block [W-Dmg].
- **tau(e)**: time to contact for each entity, distance divided by closing speed, in ticks [L76].
- **cpa(p)**: closest point of approach and ticks to it for each projectile.
- **expected(self)**: the next position, block changes and item changes caused by our own
  commanded inputs (efference copy). Anything not in it is "external".
- **harm(T)**: predicted HP loss over the next T ticks from all active hazards, used for floors.

### 4.2 Priority classes

- **P0 lethal-imminent**: predicted death or loss of half the remaining HP within the budget
  window. Ignores brain inhibition. Always reported.
- **P1 damage in progress or imminent**: contact hazards, projectiles, edges.
- **P2 threat defence**: mob stances.
- **P3 homeostatic upkeep**: food, health top-off, equipment, inventory.
- **P4 social and orienting**: look at the speaker, follow the companion, unstuck.
- **P5 goal execution**: the brain's goal, armed responses, compiled habits, skills.
- **P6 idle**: scanning, tidying.

### 4.3 Catalog

Gains referenced below (set by the brain, section 4.5): aggression a in [0,1] (default 0.5),
caution c in [0,1] (default 0.5), hp_floor (default 8 HP), hunger_floor (default 14), leash
(default 6 blocks), vigilance v in [0,1] (default 0.5). "Inhibit" means the brain can
suppress the behaviour with a scope and expiry. "Floor" means the inhibition is ignored once
harm(T) reaches P0.

#### Level 1: protective reflexes (spinal and brainstem analogues)

| ID and name | Trigger (condition on state) | Response | Budget | Prio | Brain modulation |
|---|---|---|---|---|---|
| R1 lava withdrawal | lava intersects the player box, or lava is the block under the feet | move to the nearest standable non-hazard neighbour (escape vector precomputed each tick); jump while in lava; then R3 | 1 | P0 | none; lava deals 4 HP every half second [W-Dmg] |
| R2 lava pre-contact | the next footstep (from expected(self)) or the current dig target is lava or is adjacent to exposed lava | cancel the step or dig; place a throwaway block over the lava face if one is in the hotbar; report once per lava body | 1 | P1 | inhibit scoped to a region ("I am bucketing this lava"); margin grows with c |
| R3 extinguish | on-fire flag set and not in water, no fire resistance | water within 4 blocks: step in. Else water bucket: place at feet, retrieve after 2 to 4 ticks. Else move off flammable blocks and stop sprinting | 2 | P1 | auto-off under fire resistance; burning deals 1 HP per second [W-Dmg] |
| R4 contact hazard | player box touches fire, lit campfire, cactus, sweet berry bush, wither rose, magma (not sneaking), or powder snow without leather boots | step to the nearest safe neighbour; on magma, sneak if the path must cross | 1 to 2 | P1 | inhibit scoped ("harvesting berries", "crossing magma, sneak") |
| R5 edge guard | expected(self) next tick has no support and the drop below is deeper than safe_drop (3 at c=0, 1 at c=1), or ends in lava or the void | sneak and zero forward input; if the goal requires the drop, report "goal blocked by edge" | 1 | P1 | inhibit scoped to region or goal ("bridging here", "dropping into water"); floor: never inhibited when dmg_land is lethal and there is no water or other mitigation below |
| R6 fall arrest (MLG) | airborne, falling, and dmg_land at least hp + absorption - hp_floor | pre-select water bucket (or slime block, cobweb, scaffolding, boat) as soon as triggered; pitch to straight down; use it on the landing block on the first tick that block is within 4.5 blocks of the eye [W-Attr]; retrieve water within 10 ticks of landing. If nothing usable, R11 | starts at least 6 ticks before t_land | P0 | threshold lowered by c; no inhibition when lethal. Reliability is an open risk: Baritone's author calls its version "questionable" [BAR] |
| R7 void guard | position below the world floor plus 4, or an edge whose column has no blocks to the void | treat as an infinite drop in R5; never inhibited | 1 | P0 | none; totems do not save from void damage [W-Tot] |
| R8 surface for air | head in water and air below air_floor (default 100 of 300 ticks), no water breathing or conduit power | hold jump and swim to the nearest air block found by a vertical scan; if the column is blocked, mine upward or move to an open column | 10 (P1), 1 when air at or below 20 | P1 to P0 | air_floor lowered by brain for underwater work; drowning deals 2 HP per second once air is gone [W-Dmg] |
| R9 suffocation and falling blocks | head inside a solid block, or a falling sand/gravel/concrete powder/anvil/dripstone entity in our column within 3 blocks | step horizontally out of the column; if enclosed, mine the block at head height with the best tool | 1 | P0 when suffocating, else P1 | none; suffocation deals 1 HP every half second [W-Dmg] |
| R10 dig guard | the commanded dig target is the block under our feet with unknown, air, liquid or void beneath within 3 blocks; or the block above our head with a gravity block above it | veto the dig and substitute the staircase chunk (K6) or offset by one block; report once per goal | 0 (pre-action) | P1 | inhibit per goal ("dig down here, floor is known"); floor: always veto when lava or void is detected below |
| R11 totem | a totem is in the inventory and any of: hp + absorption at most harm(20); hp at most 6; a lit creeper fuse within 4 blocks; R6 triggered without mitigation | move the totem to the offhand (swap-hands if it is in the selected slot, else inventory click); after it pops, re-equip the next one within 2 ticks | 2 | P0 | mode "always totem" keeps it in the offhand permanently |
| R12 hit from behind | damage event whose attacker or source direction is more than 90 degrees from our yaw, or attacker not visible | turn to the source (O-level), raise the shield if held, step sideways from the source; sensitise defensive gains for that mob type by 1.5x for 600 ticks | 2 | P2 | sensitisation factor and duration are gains |
| R13 sound startle | sound event of creeper priming, TNT priming, bow draw near us, explosion, warden heartbeat or sonic charge, ghast warning, within 16 blocks | orient to the source; prime the matching defence (shield, flee, cover) | 1 | P1 | ignore list (habituated sounds) |
| R14 projectile evade | projectile with cpa under 1 block and ticks to cpa at most 15 | a shield already up stays up facing it whatever the ticks left; else if a shield is available and ticks to cpa at least 5: face and block. Else strafe perpendicular to its velocity. Ghast fireball within reach and ticks at most 4: hit it back | 1 | P1 | c widens the miss distance |
| R15 knockback recovery | velocity change not in expected(self) while within 2 blocks of an edge, lava or the void | counter-input toward support and sneak; if already airborne over a drop, place a block under the feet (block clutch) | 1 | P0 or P1 | none |

#### Level 2: defensive stances by mob type (PAG and amygdala analogue)

The stance applies when the mob is within its engage distance with line of sight. Imminence
bands follow [FL88]: pre-encounter (vigilance, keep distance), post-encounter (avoid or
prepare), circa-strike (fight or flee at reflex latency). aggression a shifts the fight/flee
boundary; hp_floor forces flight regardless of a.

| ID | Mob | Default stance | Trigger and response | Budget | Prio | Brain modulation |
|---|---|---|---|---|---|---|
| D1 | Creeper | duel with a sword, else avoid | sword held or in the hotbar, a >= 0.5, hp >= hp_floor, no second creeper lit within 7: hit and retreat between fuse starts. The fuse starts inside 3 blocks and keeps burning until the target is past 7, then burns back a tick at a time; the hitbox is reachable from about 3.3. Meet it at reach with the swing ready, sprint-hit (extra knockback), sprint away from all creepers in view (inverse-square weighted) past 7 while the sword recharges, repeat; fuse estimate (ticks lit minus ticks unlit, sensed) at 18 or more: flee. Otherwise: back away to keep 7 or more blocks while facing it; fuse lit within 7: shield if held (explosions are blocked in Java [W-Sh]); else sprint directly away until 7 blocks (about 4 blocks at 0.28 blocks/tick is about 15 ticks of a 30 tick fuse [W-Cr, W-Spr]) | 1 once fuse lit | P2, P0 when lit within 3 | a; "protect area" makes it lure the creeper away from a build instead of fleeing toward it |
| D2 | Zombie, husk, drowned (melee) | fight if a >= 0.5 and hp > hp_floor, else retreat to companion | within 8: K1 melee. Three or more melee mobs within 6: K5 pillar up 3 blocks or retreat. Baby zombies: retreat starts at 10 blocks. Drowned holding a trident: treat as ranged (D3) | 2 | P2 | a, hp_floor |
| D3 | Skeleton, stray, bogged (ranged) | fight in the reload, break line of sight when hurt | line of sight is a voxel ray over the sensed blocks (eye to eye, as the mob's own check; unknown blocks count as clear). The adapter senses each archer's draw ticks and ticks since its last shot (a draw of 18 or more that ends); a skeleton looses after 20 ticks of draw and draws again 40 ticks after a shot on Normal, aiming with no lead. a >= 0.5 or nowhere to run: with a shield, sprint in during the reload and raise it toward the archer whose arrow can land within 10 ticks (5 to raise plus margin), keep it up while that arrow is in flight, lower it to strike (K1); without one, zigzag in at 50 degrees off the line, holding the side through an expected release. hp < hp_floor (until 6 above it), or no shield against 3 archers in sight: go to the nearest hidden standable cell within 5 and hold it; in the open, circle sideways at a walk (sprinting burns the food that regenerates health), shield up when an arrow is due. a < 0.5 on open ground: retreat out of range. Skeletons fire within 15 blocks and back off within 4 [W-Sk] | 2 | P2 | a; "stay in cover" mode |
| D4 | Spider, cave spider | neutral in daylight, fight at night | daytime spiders are not attacked unless they attack; at night treat as D2; cave spider (poison) retreats at higher hp_floor | 2 | P2 | a |
| D5 | Enderman | never look | gaze filter: no yaw/pitch that puts its head inside the view cone within 64 blocks [W-End] unless a carved pumpkin is worn. If provoked: retreat under a ceiling lower than its height or into water; fight only if a >= 0.8 | 0 (filter on every look) | P1 | none for the filter; a for fighting |
| D6 | Creaking | keep looking | within 16: keep it within 60 degrees of gaze with no block between (it cannot move while watched [W-Crk]) and back away; report; killing needs its heart | 1 | P1 | brain sets "go find the heart" |
| D7 | Witch | close fast or leave | within 10: if a >= 0.6, sprint in and K1; else retreat out of throw range (range unverified) | 2 | P2 | a |
| D8 | Phantom | face up and hit | phantom diving toward us: pitch up, hit at reach; or step under cover | 2 | P2 | a; H3 reports the cause (no sleep) |
| D9 | Warden | flee silently | darkness effect applied, warden visible, or shrieker/heartbeat sound: sneak (sneaking avoids vibrations, standard mechanic, not re-read) and retreat along the path we came; never fight | 1 | P0 | none |
| D10 | Piglin (Nether) | appease | entering the Nether or piglin within 16: ensure one gold armour piece is worn (H5); do not open containers or mine gold near them | 20 | P3 | brain may disable when a raid is intended |
| D11 | Neutral and friendly (wolf, bee, iron golem, zombified piglin, villager, pets, companion) | never attack | attack mask applied to every K1 swing and every arrow line of fire; if one turns hostile because of us, retreat | 0 | P1 | brain may lift the mask for a named target |
| D12 | Others: blaze, ghast, guardian, pillager, vindicator, ravager, vex, breeze, slime | table-driven | blaze and guardian: break line of sight; ghast: R14 deflect; pillager: cover; vindicator and ravager: retreat; vex and slime: K1 | 2 | P2 | a |

#### Level 3: locomotion pattern generators (CPG analogue)

The goal executor supplies direction and a speed drive (the brainstem locomotor region); these
supply the rhythm. All are P5 unless stated.

| ID | Behaviour | Trigger | Response | Budget | Modulation |
|---|---|---|---|---|---|
| C1 | gait selection | straight segment of 8 or more blocks, flat, no threat, food > 6 | sprint-jump; plain sprint when food is scarce; walk when food <= 6 (sprint impossible [W-Spr]); sneak within 1 block of an edge | 1 | "hurry" gain trades hunger for speed |
| C2 | step-up jump | 1-block rise directly ahead (step height 0.6 [W-Attr]) | jump on the tick before contact | 1 | none |
| C3 | swim | in water with the goal across or above | sprint-swim toward the goal; hold jump to surface; jump at the shore edge to climb out | 1 | none |
| C4 | climb | inside ladder, vine or scaffolding with the goal above or below | forward plus jump to climb; sneak to hold; release to descend | 1 | none |
| C5 | doors and gates | the path crosses a closed door, gate or trapdoor | open, pass, close behind at night or when hostiles are within 16 | 2 | "close doors" gain |
| C6 | unstuck | intended motion but observed displacement under 0.05 blocks/tick for 20 ticks (efference-copy mismatch) | escalate: jump; strafe left/right; back up 2 blocks; mine the blocking block at head or feet; replan; after 3 rounds (about 200 ticks) wake the brain | 20 to 60 | P4; escalation length is a gain |
| C7 | pillar and bridge rhythm | invoked by K4/K5 | the rhythm parts of the chunks | 1 | none |

#### Level 4: orienting (superior colliculus analogue)

Gaze is a separate channel from the legs (section 4.4), so the bot can look at a threat while
walking away. Classification happens only for things that win orienting; this is the
attention gate that limits what goes upward.

| ID | Behaviour | Trigger | Response | Budget | Prio | Modulation |
|---|---|---|---|---|---|---|
| O1 | look at speaker | chat from a player within 16 blocks, or any chat naming the bot | turn the head to the speaker within 4 ticks (about a human orienting saccade); hold 40 ticks unless a threat wins | 4 | P4 | none |
| O2 | looming | any entity with tau under 40 ticks and within 16 blocks | turn to it, classify, hand to D-level if hostile | 2 | P2 or P4 by class | v scales the tau threshold; habituates per mob type (chickens stop drawing gaze) |
| O3 | sound turn | positioned sound of interest (hostile steps, bow, hiss, explosion, block break near us, player hurt) | turn to the source | 2 | P2 to P4 | ignore list |
| O4 | gaze following | the companion's look ray stays on one point for 20 ticks, or they say "look", "that", "there" | look at their ray target; put "human is looking at X" in the next digest | 10 | P4 | off in combat |
| O5 | shoulder check | in a cave or at night with no threat seen for 100 ticks | glance 150 degrees behind for 6 ticks, about every 5 s | 6 | P6 | v sets the rate |
| O6 | inhibition of return | a stimulus was oriented to and classified harmless | do not re-orient to it for 200 ticks unless its class or tau changes [Kl00] | 0 | n/a | duration is a gain |
| O7 | gaze filters | always | D5 (never look at enderman heads) and D6 (keep looking at creakings) applied to every look command from any behaviour | 0 | P1 | none |
| O8 | drop spotting | item entity spawns within 8 blocks from our own mining or kill, or thrown by the companion | mark it for H7 | 1 | P5 | none |

#### Level 5: homeostatic drives (hypothalamus analogue)

Urgency rises nonlinearly with deviation from the set point [KG14]. Several drives report upward
instead of acting, because the right action is strategic (interoception [Cr02]).

| ID | Drive | Trigger | Response | Budget | Prio | Modulation |
|---|---|---|---|---|---|---|
| H1 | hunger | food <= hunger_floor, no hostile within 12, no atomic action in progress, and no archer in sight able to land an arrow within the 32-tick meal (out of sight, or in its reload); emergency if food <= 6 (cannot sprint, so cannot flee) | eat the best food (by saturation when hp < 20, since regeneration needs food >= 18 [W-Hun]); eating takes 32 ticks and pauses sprint | 20 (2 in emergency) | P3, P1 in emergency | hunger_floor; banned foods; report "food stock under 5 servings" once |
| H2 | health | hp < hp_floor with a melee threat in 16 blocks: retreat (to companion, shelter, or K5); archers are left to D3, since running from one in the open turns the back to it; hp < 20 and food < 18 and safe (including the archer meal gate of H1): top off food; hp <= 6 in combat with a golden apple and 32 safe ticks: eat it | as stated | 2 for retreat | P1 retreat, P3 top-off | hp_floor |
| H3 | sleep pressure | time since rest above 60000 ticks (phantoms come at 72000 [W-Ph]) and night | report "phantoms possible in about N minutes; bed: yes/no"; do not act | n/a | upward only | threshold |
| H4 | darkness | in an enclosed space with block light 0 at the feet (hostile spawning needs block light 0 since 1.18, not re-read) and torches in the inventory | place a torch at the feet or on a wall; about every 8 blocks in tunnels | 10 | P5 | "torching" gain; off when sneaking past mobs |
| H5 | equipment | better armour in the inventory and no threat; tool durability at or below 5 percent; enchanted tool at or below 2 percent; shield available and offhand empty; Nether without gold armour | equip armour; switch to another tool of the same kind; stop using the enchanted tool and report; put the shield in the offhand | 20 | P3 | brain lists tools it wants used to destruction |
| H6 | inventory and body schema | 2 or fewer free slots; or a hotbar slot does not hold its schema item while one is in the inventory | drop items on the brain's junk list (default: dirt, gravel, cobblestone above 64, rotten flesh, seeds); else report "inventory full" once. Restore the hotbar layout (1 sword, 2 pickaxe, 3 axe, 4 blocks, 8 food, 9 water bucket) when idle | 40 | P5/P6 | junk list and layout set by brain |
| H7 | item collection | marked drops within 6 blocks, safe path, no threat; urgency rises toward despawn (6000 ticks) | walk over them | 20 | P5 | "leave it" inhibition |
| H8 | attachment (companion) | distance to the companion > leash and the goal is not tagged solo; companion hp drops or a hostile is within 4 of them; we stand within 1 block of them while they move toward us | path to within 3 blocks; engage or warn in chat depending on a, and wake the brain; step aside | 10, 2 for danger | P4, P2 for danger | leash (0 disables); "protect companion" gain; wake the brain if more than 32 blocks apart; no follow for 200 ticks after a goal ends, so the body stays where the goal left it until the brain issues the next goal |

#### Level 6: habits and skills (basal ganglia chunks with cerebellar timing)

| ID | Skill | Contents | Budget | Modulation |
|---|---|---|---|---|
| K1 | melee micro | swing only when the cooldown is ready (sword attack speed 1.6/s, about 12.5 ticks, standard Java value); crit by jumping and striking on the way down while not sprinting; sprint hit then release forward (w-tap) for knockback; strafe at the 3-block reach boundary [W-Attr]; block between swings if holding a shield; respect D11 mask | 1 | a |
| K2 | shield timing | raise at least 5 ticks before predicted impact (bow draw, melee wind-up within 2.5 blocks); held only while commanded every tick (hold-to-block) and lowered to swing, the strike landing the next tick as a vanilla client discards attack clicks while an item is in use; movement while blocking or eating is cut to vanilla's 0.2 with no sprint, which Azalea omits and the adapter emulates; if disabled by an axe, switch to retreat for 100 ticks [W-Sh] | 1 | none |
| K3 | bow | lead the target with arrow gravity; only when the brain sets a ranged stance | 1 | brain-enabled |
| K4 | bridge | sneak, back to the edge, place a block on the edge face, repeat | 1 | used by goals; R5 is satisfied by its own sneak |
| K5 | pillar up | look down, jump, place below at the apex, repeat N times | 1 | N set by brain or D2 |
| K6 | staircase descent | dig a 1x2 stair one block forward and one down per step; never the block under the feet | 1 | substituted by R10 |
| K7 | tunnel | 1x2 tunnel with H4 torches, blocks over lava and water faces, report ores seen | 1 | brain gives direction and length |
| K8 | fell tree | all logs including the top (pillar if needed), collect drops | 1 | none |
| K9 | bucket place-and-retrieve | place water, wait 2 to 10 ticks, retrieve | 1 | used by R3, R6, lava crossing |
| K10 | compiled habits | learned from the brain's repeated decisions (section 4.6) | 1 | brain endorses or revokes |

### 4.4 Arbitration: how each tick is resolved

**Options compared.**

- **Subsumption** [Br86]: fixed layers where higher layers suppress lower wires. Fast and
  robust, but priorities are wired, there are no graded gains, and a planner cannot re-tune
  it at runtime without rewiring.
- **Behaviour trees** [CO18]: re-evaluated every tick, modular, good for sequences. Priority is
  the order of fallback children, so changing priority means restructuring the tree; gains
  have no natural place; two subtrees cannot share the body by channel.
- **Fixed-order priority with interrupts** (Mindcraft): simple, but one global winner per tick,
  binary on/off, and the fleeing-before-fighting failure shown in section 3.
- **Voting / weighted fusion** (DAMN, Rosenblatt 1997): smooth, but averaging motor commands is
  wrong for discrete game actions (half-placing a bucket is not a thing).
- **Basal-ganglia selection by disinhibition** [Mi96, RPG99; robot model in PMG06]: every
  behaviour bids a salience; each resource is released to the highest bidder; the rest stay
  inhibited.

**Chosen: priority-class plus gain-weighted salience selection, per body channel, with
hysteresis, commitment locks and floors.** Behaviour trees are still used inside individual
skills (K-level) for sequencing. The reason: the brain's knobs are gains and inhibitions, and
this is the only scheme where gains are first-class inputs to every decision; it allows legs
and gaze to be owned by different behaviours; and it degrades to plain priority when all
gains are default.

**Per tick:**

1. Sense, then run the forward model (section 4.1).
2. Each behaviour b computes urgency u_b in [0,1] from its trigger, then salience
   s_b = class_base(P) + g_b * u_b * (1 - i_b), where g_b is its gain from the current mode and
   i_b in [0,1] its brain inhibition. If harm(T) for b reaches P0, i_b is treated as 0 and an
   OVERRIDE event is raised.
3. Each behaviour declares the channels it needs: LEGS (movement keys, jump, sneak, sprint),
   GAZE (yaw, pitch), MAIN (attack, use, hotbar slot), OFF (offhand), INV (inventory clicks),
   CHAT. A behaviour may declare a degraded variant needing fewer channels.
4. Channels are allocated greedily in salience order. A challenger takes a channel only if
   its salience exceeds the holder's by a hysteresis margin (0.1) or the holder finished.
   Atomic actions (eating 32 ticks, the shield's 5-tick raise, a bucket place-retrieve)
   hold a commitment lock that only P0 breaks.
5. Motor composition: the LEGS owner expresses a world-frame velocity; the motor layer turns
   it into WASD relative to whatever yaw the GAZE owner chose. This lets D1 walk backward
   while facing the creeper, or the goal walk forward while O1 looks at the speaker. Gaze
   filters (O7) are applied last.
6. The brain's goal is one behaviour at P5 whose salience is the goal priority the brain set.
   When it loses LEGS for more than 200 ticks, or its preconditions break, the upward
   channel gets an INTERRUPTED event (section 4.5).
7. Emit events to the upward channel.

Cost: about 60 behaviours each evaluating a cheap predicate per tick is negligible next to
the 50 ms tick.

### 4.5 Upward channel: what the brain hears and when

Principle: report prediction errors, not state. The brain already holds a model (what it was
last told plus what it commanded). An event goes up only if it contradicts that model
(Sokolov's orienting reflex [So63]), passes habituation [TS66], and fits the budget.

**Three classes.**

- **WAKE** (the brain's long-poll returns at once): death or near-death (P0 fired with hp
  under hp_floor); the goal failed or is impossible; the goal was interrupted for more than
  200 ticks; human chat addressed to the bot or within 16 blocks; the companion is in danger
  or more than 32 blocks away; a brain inhibition was overridden by a floor (the brain's
  instruction was not obeyed and it must know); an armed response fired; a threat class seen
  for the first time this session (first warden, first creaking); unstuck escalation failed;
  a habit failed.
- **DIGEST** (returned on the brain's next observe call): reflex firings aggregated by type
  with counts and net HP change; newly salient entities that survived habituation; resource
  changes (tool broke, inventory full, food stock low, phantoms coming); habit candidates;
  gaze-following targets.
- **LOG** (only on request): everything, for debugging.

**Habituation.** Each event key (type, subject class, coarse place) carries a weight w that
halves on each repeat inside a window and recovers with a time constant (default 2 minutes).
A change in magnitude (more mobs, more damage) resets w (dishabituation). The brain can seed
habituation with ignore() and break it with attend().

**Rate limits and tokens.** WAKE at most once per 5 s except P0. DIGEST at most about 150
tokens; the compact state snapshot at most about 250 tokens. Example digest line format:
`t+84 lava_withdraw x1 hp 18->14 | creeper fled x2 | goal "iron" paused 40t resumed | food 9`.
Rough cost (estimates, not measured): a full state dump each brain cycle is 800 to 2000
tokens; at 12 cycles per minute that is 0.6 to 1.4 M tokens per hour, against roughly
0.1 to 0.3 M for snapshot plus digest.

**Harness binding.** Thought runs in the harness, so the executor is exposed as
tools: observe() returns the snapshot and the digest since the last call; wait_event(max_s)
blocks until a WAKE event or the timeout, so the brain can idle without polling; act tools
carry the downward knobs. When the brain is mid-turn and a WAKE fires, the event waits for
the next tool return; reflexes do not need the brain to act, which is the point.

### 4.6 Downward channel: the knobs

Seven, and no more. Everything persists until replaced or expired.

1. **goal(spec, priority)**: one active goal and a queue of up to 3. The spec is handed to the
   planner and skill library; tagged solo or with companion (affects H8).
2. **mode(name)**: a preset of gains, inhibitions and gaze settings: explore, mine, build,
   fight, retreat, follow, idle-safe. Analogous to a neuromodulatory state.
3. **gain(name, value)**: aggression, caution, hp_floor, hunger_floor, leash, vigilance.
4. **inhibit(behaviour, scope, ttl)**: scope is a region box, "until the current goal ends",
   or a duration; strength in [0,1]. Floors apply. Example: "building on this cliff face:
   inhibit R5 within box B until the goal ends"; R5 then stays off inside B unless dmg_land
   becomes lethal with no mitigation, which raises OVERRIDE.
5. **arm(trigger, response, ttl, once)**: a brain-prepared conditional response (StartReact,
   implementation intentions). Triggers come from a fixed vocabulary: entity_within(type, d),
   entity_tau(type, ticks), hp_below(x), companion_hp_below(x), sound(name),
   block_seen(type, d), chat_matches(pattern), at(box), item_below(item, n). Responses are
   catalog behaviours with parameters or a few primitives: place_block_between(target),
   water_at(target), retreat_to(pos), attack(target), use(item), say(text), goal(spec).
   Example: arm(entity_within(creeper, 6) near companion, place_block_between(creeper,
   companion), ttl 300 s). Armed responses run at P2 or the class of the behaviour they call.
6. **attend(target) / ignore(class, ttl)**: biases orienting and seeds or breaks habituation.
7. **report(level)**: verbosity and token budget for DIGEST.

Example of the user's two cases. "We are building on a cliff edge": mode(build), then
inhibit(R5, box around the build, until goal done). "Be aggressive": gain(aggression, 0.85),
which moves D2 from retreat to fight, D3 from cover to approach, and lowers the K1 swing
threshold; hp_floor still forces retreat.

### 4.7 Habit compiler (from repeated brain decisions)

The brain's repeated context-to-action decisions are the costliest thing in the loop. The
compiler turns them into stimulus-triggered habits, following chunking with task bracketing
[Gr98, J99, Ma18] and reliability-based arbitration [DND05].

- **Record** each brain-issued goal: a context signature (mode, held tool, nearby block and
  entity classes, light, time of day), the goal type, the primitive sequence executed, and
  the outcome.
- **Propose** when the same (goal type, context signature) produced the same sequence skeleton
  successfully 3 times: DIGEST carries "habit candidate: when X, do Y". Examples: "after a
  kill, collect the drops"; "leaving a cave at night, block the entrance"; "ore within 4
  blocks while in mode mine: mine it".
- **Endorse**: the brain accepts, which stores it as a persistent armed response scoped to its
  mode (no TTL). Unendorsed candidates expire.
- **Bracket**: a habit reports only its start and end in the digest, not its steps.
- **Hand back**: track reliability as a Beta posterior over success. On a failure or an outcome
  that contradicts expected(self), pause the habit and WAKE the brain ("habit X failed in
  context Y"). Below a reliability bound, the habit is retired.
- **Goal gating against slips**: a habit fires only while its originating mode or goal type is
  active, which prevents capture errors [N81] such as mining ore while following the human.

## 5. Novel Contributions

Honest framing: hybrid reactive-deliberative architectures are 35 years old (subsumption
[Br86], schema-based AuRA where the planner selects and configures reactive schemas, three-layer
architectures [Bo97, Gat 1998 book chapter]). Fast-slow agents with an LLM on top are now
common: Talker-Reasoner [TR24], SwiftSage [SS23], Figure Helix (S2 VLM at 7 to 9 Hz over an
S1 policy at 200 Hz [Hx25]), Hi Robot [HR25], PIANO [PS24], parallel planning-acting in
Minecraft [L25], Mindcraft modes [MC-modes, MC25]. None of these is claimed. The claims
below are deltas, each with a baseline and a test.

| Claim | Beats | Falsifiable test |
|---|---|---|
| C1. Graded, scoped, expiring reflex modulation by an LLM brain, with veto bounded by predicted harm (imminence floors from [M07]) | Mindcraft binary setMode; static configs of pathfinder, Baritone, auto-eat; AuRA's configuration selection is prior art for "planner configures reactive layer", so the delta is graded, scoped, expiring and floor-bounded | Cliff-build and lava-bucketing scenarios: count wrong-intent firings (reflex contradicts the brain's instruction) and deaths. Prediction: wrong-intent firings at least 70 percent lower than our own layer with knobs frozen (ablation A1), with deaths no higher than Mindcraft with the mode switched off |
| C2. Brain-armed conditional responses executed at tick latency | Brain-only reaction (the LLM must notice, decide, act: 60 to 400 ticks); Mindcraft has no equivalent | Companion-protection scenario (creeper approaches the human): response latency in ticks and success rate, armed vs unarmed. Prediction: median latency under 5 ticks armed vs over 60 unarmed; success at least 2x |
| C3. Habituating, prediction-error-gated, token-budgeted upward channel | Full state dump per cycle; Mindcraft's log flush; PIANO's bottleneck is a design statement without a habituation mechanism | Tokens per hour and missed-critical-event rate (a scripted oracle lists events the brain must hear). Prediction: at most 30 percent of the tokens of ablation A2 (no habituation) with no increase in missed critical events |
| C4. Habits compiled from the brain's own repeated decisions, stimulus-triggered, goal-gated, handed back on failure | Voyager's skill library, which keeps the LLM in the trigger loop; options and macro-actions [SPS99] are prior art for temporal abstraction, so the delta is compiling the triggering from an LLM's decisions with endorsement and reliability handback | Two-hour sessions: brain calls per task-hour and slips (habit fired out of context). Prediction: at least 30 percent fewer brain calls in hour two than hour one, with slips under 1 per hour |
| C5. The brain-stall survival curve as the evaluation of a fast-slow game agent | No incumbent reports survival as a function of planner latency | See section 6; this is a measurement contribution, not a mechanism |

Not claimed: the anticipatory reflexes (time-to-landing, tau). Meteor's AutoTotem already
predicts fall and explosion damage and Baritone plans bucket falls; ours is engineering.

**Publication assessment.** Workshop-grade as it stands (e.g. an open-world agents or games
workshop). The strongest result would be C5 plus C1: a survival-vs-stall curve where our layer
stays flat while baselines collapse, and a modulation ablation showing that without graded
floors the bot either dies (reflex off) or cannot build (reflex on). C3 and C4 need the
token and call counts to be large to matter. Novelty check is against Minecraft agents,
fast-slow LLM agents and classical hybrid robotics; the motor-control-inspired robotics
literature (e.g. basal ganglia robot selection [PMG06]) was checked only at the level of the
selection mechanism, which is why selection itself is not claimed.

## 6. Evaluation protocol (brain stall)

- **Scenarios** (fixed seeds, scripted hazards): S1 one night in open plains; S2 cave descent
  with lava and gravel; S3 ravine and cliff traversal; S4 creeper ambush; S5 skeleton field;
  S6 underwater retrieval; S7 follow the human through mixed terrain; S8 building on a cliff
  edge (tests C1); S9 creeper approaching the human (tests C2).
- **Stall**: the brain's tool responses are delayed by N in {0, 10, 30, 120} s.
- **Arms**: B0 raw primitives, no reflexes (Voyager-style); B1 Mineflayer plugin stack
  (pathfinder, pvp, auto-eat, armor-manager, defaults); B2 Mindcraft modes, defaults, setMode
  allowed; A1 ours with knobs frozen at defaults; A2 ours without habituation; A3 ours full.
- **Metrics**: deaths per hour, damage per hour, time to first death, task completions per hour,
  tokens per hour, WAKE events per hour, wrong-intent firings, missed critical events, time
  separated from the companion by more than 32 blocks.
- **Power**: at least 10 seeds per cell; report means with 95 percent intervals.
- **Headline prediction**: at N = 30 s, A3 deaths per hour at most half of B2 and at most a
  quarter of B1. If A3 does not beat B2 at N = 30, the reflex catalog is not the reason the
  bot survives and C5 fails.
- **Version risk**: the test target is Java 26.3 on `tr4m0ryp/Minecraft-rust`. Whether
  Mineflayer (4.39.0 latest on npm) speaks 26.3 was not checked; the baselines may need a
  1.21.x server, which would put the arms on different versions. Resolve before the first
  measured run.

## 7. Build order for the first session (one week)

Ranking is by estimated deaths prevented per unit of effort. There is no measured
distribution of death causes yet; the first session must log the killing damage source of
every death, and the order is re-ranked from that log. Effort: S about half a day, M about a
day (estimates).

Step 0 (prerequisite, not ranked): tick loop, state snapshot, own-motion forward model and
expected(self), per-channel arbiter with gains and inhibit, event log with observe() and
wait_event(). Including gains and inhibit from day one costs little and lets the first
session exercise C1.

1. **R1 + R4 + R3 contact hazards** (lava, fire, cactus, magma, extinguish without bucket), S.
   Lava is the classic bot killer and the response is a single step.
2. **R5 + R7 edge and void guard**, S. Uses expected(self); stops the commonest pathing death
   in caves and ravines.
3. **R10 + R9 dig guard and falling blocks**, S. A pre-action veto; nearly free once R5 exists.
4. **R8 surface for air**, S.
5. **H1 + H2 eat and top off, with the food <= 6 emergency**, S. Keeps regeneration and the
   ability to sprint away.
6. **D1 creeper + O2 looming orient**, M. The most damaging common mob; needs fuse detection.
7. **D2 + K1 melee defence with hp_floor retreat, plus D5 enderman gaze filter and D11 attack
   mask**, M. Night survival; the mask protects the human.
8. **H8 + O1 companion leash and look at the speaker**, S. Not a death preventer, but the
   project's purpose is playing alongside the human.
9. **C6 unstuck by efference-copy mismatch, with WAKE escalation**, S to M. Prevents silent
   stalls that leave the bot exposed at night.
10. **R11 totem plus R14/K2 shield and strafe against arrows (with D3 cover)**, M.

Deferred to week two: R6 water-bucket clutch (high effort, reliability doubtful, needs tick
tuning against the server), K5 pillar escape, the habit compiler (C4), the full arm()
vocabulary beyond two or three triggers, H3 sleep pressure, D6 to D12.

## 8. Unverified or open items

- Exact millisecond values for the acoustic startle (Brown et al. 1991) and the nociceptive
  RIII reflex were not read in primary sources; the table marks them.
- Hammond 1956 and Carpenter et al. 2004 are cited through secondary sources.
- Vertical drag 0.98, sword attack speed 1.6, warden vibration rules, and the 1.18 block-light
  spawning rule are standard values not re-read for this note.
- Critical-hit conditions and w-tap mechanics come from mirrors and mod pages.
- The LLM decision latency (3 to 20 s) and all token figures are estimates to be measured in
  the first session.
- Water landing timing, measured on the 26.3 server (tools/live/staged/mlg.py, Resistance IV,
  2 drops per cell): water used on the last tick before impact (predicted t_land = 1, eye 2.7
  to 2.85 blocks from the floor) cancelled all fall damage from 20 and 45 blocks (4 of 4);
  2 ticks out it saved at 20 blocks (eye 4.30) and missed at 45 (eye 4.76, out of the 4.5
  reach); 3 to 6 ticks out always missed. The window is reach-bound, not tick-bound: R6
  places on the first tick with t_land <= 2 and the floor within 4.4 of the eye, else on
  t_land = 1. Near terminal speed (3.9 blocks/tick) a tick inside reach exists only about
  three times in four, so very long falls stay unreliable.
- Knock-off recovery, measured on the 21x21 platform (tools/live/staged/knockoff.py, bot held
  0.35 from the edge, zombie hitting from inside): a zombie push moves the player only about
  0.45 blocks, so a knock-off takes several hits. Air-strafe alone (R15 airborne) saved 0 of
  8; adding the on-ground step back once the centre is past the edge saved 7 of 7 valid
  trials over about 19 hits each; the block clutch (R6, cobblestone) saved 8 of 8.
- Staged survival (tools/live/staged/survival.py: 30x30 walled arena, frozen midnight, iron
  sword and 8 bread, brain absent, 4 waves of 225 s; run in parallel arenas by batch.py, seeds
  1 to 5, mean +- sample sd). The earlier one-run figures (melee-only 16 deaths, full 7) are
  replaced by a 5-run noise floor of the same build: melee-only (D2 alone) 17.4 +- 1.5 deaths
  and 593 +- 47 HP lost; full catalog 7.2 +- 1.5 deaths and 271 +- 55 HP, of which skeletons
  4.6 +- 0.5 deaths (173 +- 32 HP), mix 1.8 +- 1.3, creepers 0.8 +- 0.4, zombies 0. After the
  archer and creeper work (D3 reload-timed shield and zigzag, line of sight and cover, meal
  gating, H2 leaving archers to D3, hold-to-block shield, item-use slowdown, D1 duel), the
  full four-wave run gives 0.6 +- 0.9 deaths and 75 +- 21 HP (5 runs). Single waves from a
  fresh kit, 5 runs each, before (old build) vs after:

  | Wave | Arm | Deaths before | Deaths after | HP lost before | HP lost after | Hits before | Hits after |
  |---|---|---|---|---|---|---|---|
  | skeletons | no shield | 4.6 +- 1.1 | 0.0 +- 0.0 | 193 +- 38 | 29 +- 10 | 56 +- 9 | 8.2 +- 2.5 |
  | skeletons | shield | 4.6 +- 0.9 | 0.0 +- 0.0 | 176 +- 27 | 10 +- 6 | 51 +- 8 | 2.4 +- 1.5 |
  | mix | no shield | 2.0 +- 0.7 | 0.0 +- 0.0 | 86 +- 28 | 29 +- 6 | 22 +- 9 | 9.0 +- 0.7 |
  | mix | shield | 1.6 +- 1.5 | 0.0 +- 0.0 | 70 +- 64 | 15 +- 8 | 20 +- 18 | 4.6 +- 2.2 |
  | creepers | no shield | 0.6 +- 0.5 | 0.0 +- 0.0 | 23 +- 16 | 0 +- 0 | 1.2 +- 0.8 | 0 +- 0 |
  | skeletons, 4 pillars | no shield | 2.4 +- 1.7 | 0.2 +- 0.4 (10 runs) | 108 +- 52 | 24 +- 13 | 31 +- 13 | 6.8 +- 3.6 |

  Kills per wave after: all 4 skeletons, all 5 creepers (0 before), 6.8 to 7 of 7 in the mix.
  The old build with a shield never lowered it once raised and, with Azalea's missing
  item-use slowdown, walked at full speed behind it, which is why its shielded mix arm
  sometimes took no damage at all; the shield was otherwise of no use to it (skeletons
  4.6 deaths either way).
  Cover (D3 hides only below hp_floor, or against 3 archers without a shield) is not shown to
  help here: in the pillar arena it held the legs on under 1 percent of fight ticks, and two
  variants did no better than the default (5 runs each): cover against 2 archers in sight
  0.2 +- 0.4 deaths, 24 +- 10 HP; recovery from hp 12 instead of 8, 0.4 +- 0.5 deaths,
  32 +- 11 HP. Fighting in the reload already keeps health above the cover threshold, so the
  cover path is exercised by unit tests only; a scenario that drives health low with
  archers is needed to measure it.
- Fast ticking is not valid for this bot. Azalea's client loop runs `GameTick` on a fixed
  50 ms timer and leaves the ticking-state packet unhandled, so with `/tick rate` at 40, 80
  and 100 the bot still ticked 20.0 times a second (tick gaps p50 55 ms, p99 61 to 69 ms)
  while the server held 2.6 to 6.2 ms per tick: mobs, arrows and fuses ran 2x to 5x faster
  than the bot's reflexes, and every reflex latency counted in server ticks scaled with the
  rate. One full arena run at rate 40 with wall-clock waves halved gave 11 deaths and 424 HP
  lost (skeletons 6, mix 5) against the 1x noise floor of 7.2 +- 1.5 deaths and 271 +- 55 HP:
  outside run-to-run noise. Throughput comes instead from parallel arenas
  (tools/live/staged/batch.py: up to five bots, one walled arena each, 100 blocks apart, at
  20 Hz; server about 10 ms per tick and bot tick gaps unchanged with four running).
- Mindcraft profile defaults for cowardice vs self_defense were not checked.
- Movement that changes the world, measured on the goal site (tools/live/staged/goals/
  terrain.py, iron pickaxe and 32 dirt, 5 trials per case, old build against new). The
  old planner (Azalea's A* with mining off and walking moves only) reached 0 of 35:
  every case ended "Unreachable" within 2 to 8 s. The new route planner
  (reflex-core goals/travel/route: walk, step, drop, dig through natural blocks, staircase
  up and down, pillar, bridge by sneaking to the edge; walking-only search first, so a
  walkable route always wins) reached 40 of 40: sealed 2-high pocket with a go_to 5/5
  (10.6 s), the same pocket with no goal at all 5/5 by the escape drive (14 to 16 s; the
  old build has no escape), 6-deep pit 5/5 (9 to 13 s), 4-block gap 5/5 (2.5 to 4 s),
  6-high ledge 5/5 (5 to 6.5 s), 3-thick wall 5/5 (8.6 s), come_here to the companion 38
  blocks away over a ditch, a 4-high wall and a mound 5/5 (15 to 17 s), give across the
  same terrain 5/5 (3 of 3 planks arrived each time).
- Mining yield (tools/live/staged/goals/building.py: dirt over stone, `mine stone 16`, 5
  trials): the old goal dug down into the stone under it and picked up 0 of 80 broken
  stone (server mined statistic against cobblestone in the inventory); mining level with
  the body and stepping into each gap picked up 80 of 80 (30 s per goal).
- Roof above reach (same script: 5x5 plank room, walls 2 high, roof cells 3 above the
  floor, one fill goal): the old build placed 0 of 25 per trial ("no face ... is
  visible"); the new place skill, which stands where its eye sees a support face and
  pillars there when needed, placed 25 of 25 in 5 of 5 trials (13 to 14 s) and left no
  scaffold standing.
- Idle at night (tools/brain/measure/idle; no goal = seconds per minute with no goal
  running; idle wait = seconds per minute blocked in wait_event with no goal). The 58-minute
  solo relay (24.6 night minutes, 33.3 day): night 37.1 s no goal and 25.4 s idle wait per
  minute, day 39.0 and 26.5. Causes found in its logs: the night flag froze at its login
  value (26.3 sends the clock only on login and changes), goal outcomes waited behind the
  5 s wake gap, and the thought layer waited with no goal after a journal edit or a mode
  change. After the fixes (clock run on at the server's rate, goal outcomes waking at once,
  the first wait after a goal ends returning at once, waits capped at 45 s, a night work
  list in the prompts) a 19.6-minute relay over two nights (16.7 night minutes): night 9.1 s
  no goal and 0.2 s idle wait per minute, day (2.9 minutes) 13.5 and 0.2.
- That 20-minute relay (empty inventory near spawn, as the solo hour began, but started at
  dusk with the day between the nights skipped): 65 goals, 38 done, 25 failed, 0 deaths,
  0.108 USD per minute. Failures: 15 crafts with missing ingredients or no table near, 4
  places into a cell held by leaves or dirt, 3 collects with no drops (wool), 2 missing
  tools, 1 go_to stalled; none "unreachable" (the solo hour had 41 of 127). Stone: 110
  mined, 110 cobblestone picked up (solo hour: 81 and 11). It did not attempt a roof.
- Crafting from the full recipe set (tools/live/staged/goals/crafting.py, fixed kits, no
  table or furnace at the site, 5 trials per case, item counts read over RCON). The book is
  read at bot start from the server jar on disk: 1307 recipes and 203 item tags, nothing of
  it written to the repository. Stone pickaxe from 2 logs and 3 cobblestone 5/5 (one table
  crafted and put down each time), iron pickaxe from 3 raw iron, 1 coal, 3 logs and 8
  cobblestone 5/5 (table and furnace), chest 5/5, 3 doors 5/5, 4 torches 5/5 (2x2 grid, no
  table), smelt 2 iron ingots 5/5; the impossible case (1 cobblestone) failed once with the
  same shopping list word for word 5/5 ("Get 2 any log, 2 cobblestone/blackstone/
  cobbled_deepslate"). 35 of 35, run twice (the merged build and the final one). The first
  loader read 1275 recipes and skipped 767 jar entries without saying which. By kind:
  stonecutting 351, brewing 279, transmute 33, blasting 25, smithing trim 18, banner
  duplicate 16, smithing transform 12, campfire 9, smoking 9, armour dye 6 and 9 single
  special recipes. Transmute (dyeing a shulker box or a bundle, two items in the grid) is
  now read, 32 more recipes: `craft black_shulker_box` from a shulker box and black dye 5/5,
  before 0/5 ("no recipe makes black_shulker_box"). 735 stayed skipped and the load line
  names them: 394 are another station for a result the grid or the furnace already makes
  (every cooked result has a smelting recipe; every cut one a crafting recipe, except
  cobblestone and cobbled deepslate, which are mined), 309 needed a brewing stand or a
  smithing table, and 32 give a result that depends on colour, pattern, damage or
  contents, which item names do not carry.
- Brewing stand and smithing table (tools/live/staged/goals/stations.py, fixed kits, no
  station at the site, 5 trials per case, counts read over RCON, potions by their
  contents). 291 of those 309 recipes are read now, 1598 in the book: 279 brewing and the
  12 smithing transforms (the netherite upgrades); the 18 armour trims stay out, since a
  trim leaves the item's name as it was, and 444 entries are skipped in all. A potion is
  one item name per content (water_potion, healing_potion, healing_splash_potion), in the
  recipes and in every inventory read. The planner treats both stations as it treats the
  furnace (found within 16 blocks, else a held one put down, else crafted first), counts
  one reagent for up to three bottles, and asks for blaze powder only for a stand it has to
  put down or has seen come back unbrewed; the body offers a held powder to an empty fuel
  slot on every load. A batch brews with the menu closed and is collected after 440 ticks.
  Upgrade a diamond pickaxe with a template and a netherite ingot, the smithing table and
  its crafting table made from 2 iron ingots and 2 logs: 5/5 (1 netherite pickaxe, 0
  diamond pickaxes left). An awkward potion from a water bottle and nether wart, then a
  potion of healing from it and a glistering melon slice, two goals on one blaze powder:
  5/5. Three water bottles to three potions of healing in one goal, two batches: 5/5. The
  impossible cases (the pickaxe with no template and no ingot; the potion with no wart and
  no melon) failed once each with the same list word for word, 5/5 and 5/5. 25 of 25. No
  packet patch was needed: the pinned library opens both menus and its slot numbers match
  the server's. Before: 0 of 25 by construction ("no recipe makes netherite_pickaxe").
  Not done: filling a bottle at water, the blast furnace, smoker, campfire and stonecutter
  (394 recipes whose results the grid or the furnace also makes), and trims. Two
  weaknesses seen: a count of healing_potion also counts strong_healing_potion (names
  match by their last words), and the shopping list for a deep chain names odd steps
  ("smelt 4 gold_ingot from 4 gold_nugget").
- Placing into an occupied cell (tools/live/staged/goals/protect.py): leaves, dirt and grass
  dug out of the cell first, 15 of 15; a block the player placed refused without `replace`
  and replaced with it, 5 of 5.
- Animal drops (tools/live/staged/goals/drops.py): `collect white_wool` with a sword kills a
  sheep 5/5; with shears it shears and the sheep lives 5/5 (1 to 3 wool); `collect beef 2`
  hunts cows 5/5 (2 to 4 beef); no sheep in range fails with the reason 5/5.
- Protection of the player's own blocks (protect.py, protect_hut.py, protect_tool.py; a
  5x5 dirt hut, 57 cells). Hut built by the body: go_to out 5/5, the no-goal escape drive
  5/5, the same after a bot restart (ledger from disk) 5/5, 0 hut cells broken in 15; a
  dirt wall the companion placed across a corridor stood 5/5 (go_to failed instead of
  digging it). A hut built by RCON (not in the ledger) marked with the protect tool over the
  real tool path: 0 cells broken 5/5, and 5/5 again with the bot restarted between protect
  and go_to (boxes now persist in the ledger file); the unprotected control broke 2 cells
  in 3 of 3. The restart case lost the box once in 5 when the bot was killed within a few
  ms of the tool's acknowledgement (the file was written after the ack). Killed the instant
  the acknowledgement is read (protect_tool.py, 10 trials): that build had the box in the
  file after the kill 0 of 10 and broke 2 hut cells in 10 of 10; with the write moved
  before the reply (a commit step between applying a tick's requests and answering them)
  the box is in the file 10 of 10 and 0 cells broke in 10.
- The escape drive at 20 trials (protect_hut.py hut_escape, the same script as the 5/5
  above). It gave up in 4 of 20 (trials 5, 9, 13 and 17, each followed by a 2-minute
  cooldown), 0 hut cells broken. Cause, from the flight recorder: the script ends a trial
  when the server shows the body outside the hut footprint, about half a second before the
  escape is done, and teleports it back in. The walker followed its old route into the wall
  for 100 ticks before the stall watch fired, and stalls were counted across separate
  escapes because the count was reset only by a block target's arrival, never by the sky
  target's: every third stall ended an escape. Now a body moved by anything but its legs
  (more than 2 blocks along the ground or 4.5 up or down in one tick) re-plans that tick,
  and the count resets on every arrival: 0 gave up in 20, 0 cells broken. 5 of the 20
  re-plans still stalled once: the script refills the tunnel in the tick it teleports, the
  plan made that tick still saw the old tunnel, and the walk into the refilled floor took
  the 100 ticks of the stall watch before the next plan dug. With each trial waiting for
  the finished escape (hut_escape_full) both builds leave 20 of 20 (12 to 16 s before, 6 to
  16 s after), so the escape itself was sound.
- Line of sight for the hands (tools/live/staged/goals/sight.py, 5 trials per case, block,
  sleeping state and positions read over RCON). The server takes a click on any block in
  reach, also behind a wall. Before: a block placed into a closed bedrock shell 5/5, a
  bed inside one slept in 5/5; with a 5-wide wall and a way round, placed 5/5 and slept 5/5
  from behind the wall. After (a voxel ray from the eye to the middle of the clicked face
  or bed top, over the world's blocks): shell cases refused 5/5 and 5/5 with the reason
  (placed 0, slept 0); with a way round the body walks round first, placed 5/5 and slept
  5/5 from beyond the wall; the open cases 5/5 and 5/5 on both builds.
- Building under that rule (protect_hut.py with HUT_REBUILD=1: walls and roof of the 57-cell
  hut by five fill goals, 5 huts). The first try stalled 2 huts in 4 on the roof: the body
  stood on its own scaffold block in a corner, in the cell to fill next, with its new roof
  at head height on every open side. A placement inside a fill also dropped its scaffold
  list when it finished, so the fill never took that scaffold down. Now the fill keeps the
  list and a body boxed in on its scaffold takes it down first: 5 of 5 huts complete (57 of
  57 cells, 2 scaffold blocks taken down each). Not solved: the roof's edge cells are
  only in sight from outside, and the body, roofed in by then, leaves by digging under its
  own wall (7 floor blocks over the 5 huts).
- "Only raw food" wakes (tools/live/staged/goals/hunger.py: raw beef only, a hunger effect
  that costs a food point every 2 s, 4 minutes). No play log holds the event (0 in the
  16.8-minute relay log and in the session logs), so the rate was staged. Before: 23
  wakes for 23 raw meals in 4.0 minutes, one per meal, because each meal at the
  emergency level lifts the bar above it again with raw food still the only food. After
  (one spell lasts until the body is fed, the raw food is gone or ready food turns up; one
  wake per 5 minutes, a new spell sooner than that is a digest line): 1 wake and 0 digest
  lines for 23 raw meals in 4.0 minutes.
- RCON from two clients at once is not safe on this server: 9,678 of 159,503 block tests
  came back empty with a second client sending (0 of 178,938 alone). A scan run beside a
  staged trial reported 5 furnaces and a crafting table that do not exist and a hut cell
  that was not broken. Staged scripts and ad hoc queries now run one at a time.
- Flight recorder (tools/live/staged/goals/descent.py). An unreachable go_to (bedrock shell)
  failed and printed its dump 5/5 (10 to 33 TRACE lines). A body pulled back to its start
  every tick: the first build dumped 1 of 5, because each unstuck nudge (every 40 ticks)
  forced a re-plan and both the recorder's stall watch and the walker's scrape count
  restarted on Planning; planning ticks now neither count nor reset, and it dumps 5 of 5
  after 60 still ticks (4.1 s). Cost: 28.7 ns per tick to record, 3.9 us for a full
  simulated tick with the recorder, 65 us to render a full ring.
- 15-minute solo relay on the final build (empty inventory near spawn at sunrise, one
  session, 16.8 minutes with the handover, 81 turns, 0.79 USD, 0.047 USD per minute): 24
  goals, 23 done, 1 failed (mine diamond_ore: none within 32 blocks), 0 deaths. Tool
  ladder: wooden pickaxe at tick 1449, stone pickaxe at 2604, iron pickaxe at 6091 (about 5
  minutes), then an iron sword, a bucket and a full iron armour set; 10 crafts and 5 smelts
  planned by the goals themselves, none failed. Idle: no goal 2.2 s per minute at night (6.2
  minutes) and 3.7 by day (10.6), waiting with no goal 0.0. Against the 20-minute relay
  above: 25 failures in 65 goals, 15 of them crafts.
- Wakes on a blocked goal (tools/live/staged/goals/wakes.py, 5 trials per case, WAKE lines
  counted in the bot log between the goal and its outcome, the outcome checked on the
  server). In the recorded play Thought answered a goal_blocked wake by waiting in 46 of
  52 cases (9.7). The 6 others: in 4 the goal ended by itself 4 to 16 ticks later (a done or
  failed event, which wakes anyway), 1 was a body standing in the cell it had to fill for
  750 ticks, and 2 were answered by inhibiting the edge guard R5. The rule now: a block the
  body is working around (a re-plan, a skipped block, a step aside) is a digest line; a
  wake follows only when blocks keep coming for 200 ticks with no progress in between, when
  the reason is one only Thought can clear (nobody to follow, monsters round the bed, a
  target that cannot be reached), or when R5 has held a goal's legs for 60 ticks. Before
  and after: a go_to into a bedrock shell 10 wakes and 5 (5 blocked wakes became 5 digest
  lines; the 5 failures still wake), mining with two ore blocks sealed in bedrock and two
  in the open 15 and 5 (10 blocked wakes became 10 digest lines, 2 of 2 open ores mined
  5/5 on both builds), follow with the companion out of sight 5 and 5. 30 wakes became 15
  over the 15 trials. The 200-tick rule and the R5 rule are covered by unit tests only: no
  staged case holds a block that long.
- Decision log (brain-link decisions; tools/live/staged/goals/decisions.py over the real
  tool path). Every set_goal, set_mode, set_gain, inhibit, arm, say and protect that
  arrives is one JSON line in logs/play/decisions.jsonl (a staged run writes its own file):
  the tick, the state taken from that tick's snapshot before the call changes anything
  (health, food, position, time of day, mode, entities, companion, the whole inventory by
  item name, the running goal with its progress and plan step, the queued goals, the last
  12 wake and digest events), the call with every parameter, and the body's answer. Live:
  8 calls gave 8 lines, 28 of 28 checks against the server (position within 1 block of
  RCON at each call, inventory equal to the server's at 7 of 7 calls compared, the stone
  pickaxe absent before the craft and present in the first line after it, the running
  craft goal shown with its plan). Cost on the tick thread, measured by the bot: median
  71 and 93 us over two runs of 8, at most 125 us, about 1.0 to 1.2 kB a line; a full line
  (40 entities in range, 43 item kinds) costs 46 us on average in a release build. The log
  is flushed to the operating system per line and not synced.
- Ledger timing (tools/live/staged/goals/ledger_kill.py: one dirt block placed by a goal,
  the bot killed with SIGKILL the instant it prints the confirmation, 10 trials). The file
  was written only at the audit every 20 ticks: the placed cell was in the file after the
  kill 0 of 10. Written in the tick the server confirms the block: 10 of 10. A write
  rewrites and syncs the whole file: 6.3 ms with 2000 cells in it (debug build), on ticks
  that confirm a block only.
- PIANO's reflex modules and the parallel planning-acting paper were read at abstract or
  summary level only.

## 9. Intuition layer: decision-model benchmark (bench/decider)

The Intuition layer sits between the reflexes (50 ms tick) and Thought (seconds): a small
local model that picks the next action from a one-line game state. Bar for "usable": median
under 500 ms per decision and accuracy clearly above the trivial baselines. Machine: Xeon
E5-1410 v2, 4 cores / 8 threads, AVX without AVX2, no GPU. Run of 2026-11-15, replacing the
first run of 2026-11-14 that was taken while builds were compiling.
A second run on 2026-11-15 trained the small models on 470 recorded Thought decisions (9.7):
latency passes, accuracy does not, and the layer stays empty.

### 9.1 Conditions

- Every timed run sits behind bench/decider/idle.py: it refuses to start with a compiler
  running and records the load, then samples the CPU used by other processes during the run.
  No compiler ran during any kept run (58 runs).
- The load-average target (under 0.5) was met in 0 of 58 runs. The desktop keeps about 0.8
  of a core busy (a chat client renderer at 0.4, browsers, other sessions); the 1-minute
  load read 1.5 to 5.5, part of it the previous benchmark run decaying. The game server took
  0 to 1 % of one core. Runs where other processes averaged more than 1.0 core were redone;
  rows still above that are marked "!".
- The first Laya launch of the day died with an illegal-instruction signal at start-up,
  before any timing. 13 later launches did not; cause unknown.

### 9.2 Trivial baselines

| cases | n | uniform chance | always first option | majority per kind | keyword rule |
|---|---|---|---|---|---|
| original, every 3rd (the first run's set) | 20 | 6.75 | 13 | 7 | n/a |
| original, all | 60 | 20.75 | 40 | 18 | n/a |
| goals (new) | 65 | 5 | 5 | 5 | 63 |

- The original cases list the right answer first in 21 of 25 choice cases, so "always the
  first option" scores 13 of 20. Every option-order-sensitive result below uses a fixed
  per-case shuffle for that reason.
- The goal cases (bench/decider/cases_goals.jsonl) are 65 states over the 13 real goals, 5
  each, one fixed question. The keyword rule (bench/decider/trivial.py) was written by the
  author of those cases after writing them: an upper bound for a rule, not an independent
  result. It shows that these cases, mostly driven by a chat line, are easy for a rule.

### 9.3 Baseline rerun (same settings as the first run: every 3rd case, 20 calls)

| model | threads | reps | p50 ms, mean (min to max) | p90 ms (range) | correct | first run p50 ms |
|---|---|---|---|---|---|---|
| Laya 421M | 8 | 3 | 1796 (1785 to 1816) | 1920 to 2216 | 8/20, 8/20, 8/20 | 1889 |
| Kev 0.8B | 4 | 3 | 2441 (2412 to 2490) | 2747 to 2777 | 8/20, 8/20, 8/20 | 3636 |
| Jeff router | 8 | 3 | 5929 (5809 to 6028) | 6192 to 6858 | 2/4, 2/4, 2/4 (n=3 calls) | 38255 |

- The compiling builds had inflated Jeff 6x and Kev 1.5x; Laya was unaffected. The verdict
  did not change: slowest-to-fastest 5.9 s to 1.8 s, 3.6x to 12x over the bar.
- Accuracy is identical in every repetition and identical between Kev and Laya by kind
  (choice 4/9, yes/no 3/6, score 0/3, rank 1/2): 8/20 against 6.75 by chance and 7 by
  majority. Not above the floor.
- Why 8/20: Laya answers "no" to every yes/no case (probability 0.10 to 0.17) and the middle
  level to every score case, but its raw values rank the cases correctly. With one yes/no
  threshold (0.168) and one score cut (0.911) fitted on the 40 cases outside the test set it
  scores 12/20. The shipped checkpoint warns about invalid temperatures; the fault is
  calibration, not ranking.
- Threads: Laya 2238 / 1422 / 1796 ms at 2 / 4 / 8 threads; Kev 3654 / 2441 / 2461. Four
  (the physical cores) is best; the first run's default of 8 cost Laya 26 %.

### 9.4 Variants (open small models through llama.cpp, bench/decider/logit_run.py)

Cold = whole prompt evaluated, about 149 tokens. Cached = question and options held in the
KV cache, only the state suffix evaluated (about 86 tokens). 4 threads, n = 60 timings per
row unless stated. Accuracy by argmax over the option labels, original 20 cases, shuffled.

| variant | weights | correct /20 | cold p50 / p90 ms | cached p50 / p90 ms |
|---|---|---|---|---|
| SmolLM2 135M | 4-bit | 6 | 744 / 861 | 423 / 513 |
| SmolLM2 135M | 5-bit | 6 | 861 / 974 | 490 / 552 |
| SmolLM2 135M | 8-bit | 6 | 572 / 686 | 332 / 375 |
| SmolLM2 135M | 16-bit float | 6 | 433 / 498 | 257 / 284 |
| SmolLM2 135M, 2 threads | 16-bit float | 6 | 812 / 915 | 469 / 520 |
| SmolLM2 135M, 8 threads | 16-bit float | 6 | 588 / 808 | 402 / 509 |
| SmolLM2 360M | 4-bit | 5 | 2080 / 2390 | 1185 / 1323 |
| SmolLM2 360M | 5-bit | 5 | 2354 / 2750 | 1333 / 1546 |
| SmolLM2 360M | 8-bit | 5 | 1525 / 1741 | 908 / 1020 |
| SmolLM2 360M | 16-bit float | 5 | 1106 / 1295 | 648 / 728 |
| Qwen2.5 0.5B | 4-bit | 5 | 1976 / 2339 | 1201 / 1379 |
| Qwen2.5 0.5B | 5-bit | 7 | 2284 / 2634 | 1384 / 1516 |
| Qwen2.5 0.5B | 8-bit | 6 | 1563 / 1777 | 928 / 1064 |
| Qwen2.5 0.5B | 16-bit float | 7 | 1132 / 1297 | 677 / 766 |
| Qwen2.5 0.5B, 2 threads | 16-bit float | 7 | 2139 / 2456 | 1262 / 1429 |
| Qwen2.5 0.5B, 8 threads | 16-bit float | 7 | 1458 / 1612 | 921 / 1082 |
| Gemma 3 270M | 16-bit float | 6 | 384 / 432 | 231 / 261 |
| Qwen3 0.6B | 8-bit | 5 | 2282 / 2702 ! | 1344 / 1638 ! |
| Qwen2.5 1.5B (n=20) | 4-bit | 9 | 3677 / 4256 | 2318 / 2458 |
| 8 solved examples in the cached prefix, 135M | 16-bit float | 9 | n/a | 436 / 485 |
| same, only volatile state fields evaluated (37 tokens) | 16-bit float | 9 | n/a | 189 / 229 |
| same, Qwen2.5 0.5B (36 tokens) | 16-bit float | 9 | n/a | 395 / 481 |
| sentence encoder 33M (bge-small), zero-shot cosine | float32 | 7 | 72 / 98 | n/a |

- Generated text against argmax (4-bit rows): writing the answer as text and parsing it
  gave 0/20 (135M), 3/20 (360M), 5/20 (0.5B) at 749 / 2184 / 2126 ms; argmax over the same
  prompt gave 6, 5, 5 at 744 / 2080 / 1976 ms. Argmax saves 1 to 7 % of the time and always
  returns a valid option; it does not make a model that does not know the answer know it.
- On all 60 original cases: 135M 18, 360M 13 (!), 0.5B 27 (!), 1.5B 26, encoder 29, against
  20.75 by chance. LFM2 350M did not run (decode error in this build of the library).
- Subtracting the logits of a content-free state (calibration) moved the 20-case scores by
  -2 to +3: noise at this n.

Goal cases (65, one fixed question, 13 solved examples in the cached prefix, score = first
token of the goal name, calibrated; "volatile" evaluates only HP, hunger, nearby, companion
and chat, 43 tokens, which holds when nothing else changed since the last decision):

| variant | correct /65 | p50 / p90 ms | n |
|---|---|---|---|
| 135M, no examples, letters | 6 | 324 / 360 | 65 |
| 135M, examples, full state suffix | 37 | 519 / 608 | 65 |
| 135M, examples, volatile suffix | 33 | 245 / 309 | 195 |
| 360M, examples, full state suffix | 41 | 1136 / 1347 | 65 |
| 360M, examples, volatile suffix | 45 | 534 / 663 | 195 |
| 0.5B, examples, full state suffix | 36 | 1017 / 1139 | 65 |
| 0.5B, examples, volatile suffix | 36 | 485 / 581 | 195 |
| 1.5B 4-bit, examples, full state suffix | 54 | 2804 / 2975 | 65 |
| 1.5B 4-bit, examples, volatile suffix | 52 | 1445 / 1784 | 195 |
| sentence encoder, nearest of the 13 examples | 30 | 74 / 96 | 195 |
| sentence encoder, zero-shot cosine | 18 | 74 / 96 | 195 |
| keyword rule (author-written) | 63 | under 1 | 65 |

### 9.5 Verdict against the bar, per variant

- Original 20 cases: no variant passes. Latency passes for 135M and 270M models (cached
  231 to 257 ms, cold 384 to 433 ms) and for the encoder (72 ms); accuracy never clears the
  floor (best 9/20 against 6.75 by chance and 7 by majority). 20 cases is small: the 95 %
  interval of 9/20 is 23 to 68 %.
- Goal cases: two variants are under 500 ms and far above chance and majority (5/65): the
  135M model with cached examples and a volatile suffix (33/65 at 245 ms, p90 309 ms) and
  the encoder with nearest example (30/65 at 74 ms). Both are wrong on about half the cases
  and far below the keyword rule (63/65). Not adopted. The layer stays empty.
- (a) Quantized weights: lost a comparison. On this CPU 4-bit is 1.7x to 1.9x slower than
  16-bit float and 5-bit is the slowest format; accuracy is the same within 2 cases. Premise:
  AVX without AVX2, where the integer kernels have no wide path. Reopen on any AVX2 CPU or
  GPU, where the order normally reverses.
- (b) Argmax scoring: works as a mechanism (one pass, always a valid option), worth 1 to 7 %
  latency. It was never the bottleneck: prompt evaluation is.
- (c) Prefix caching: works. 1.7x on the original layout (149 to 86 tokens evaluated), 2.1x
  to 2.3x more with volatile-ordered state fields (86 to 37 tokens). It also makes solved examples free per
  decision, and those are what moved accuracy (135M on goals: 6/65 to 37/65).
- (d) Threads: works, use 4. 2 threads cost 1.9x, 8 threads 1.3x to 1.4x.
- (e) Smallest sizes: latency passes at 135M to 270M and at the 33M encoder; accuracy lost
  the comparison to the floor on the original cases.
- Kev, Laya, Jeff as shipped: lost a comparison, to the bar (2.8x to 12x too slow at their
  best thread count) and to the floor (8/20). Not proven closed: Laya ranks correctly and
  reaches 12/20 with fitted thresholds; a smaller or exported Laya was never tested.
- What would have to be true: accuracy grows with size on the goal cases (33, 45, 54 of 65
  at 135M, 360M, 1.5B) while 500 ms on this CPU buys 135M. The 1.5B model needs 2.9x to
  5.6x more speed, which is an AVX2 or GPU machine, or the small model needs training on
  Ultron's own decisions instead of 13 examples (tested in 9.7).

### 9.6 Open after the first run

- Fine-tune or probe-train the 135M model or the 33M encoder on recorded Thought decisions:
  tested in 9.7.
- Laya with fitted thresholds on more than 20 cases, a base-size Laya checkpoint, and an
  exported integer build of it (never tested).
- Goal cases without a chat line, where a keyword rule has nothing to read: written and
  measured in 9.7 (the rule gets 9 of 48 there, and 4 of 108 real decisions).
- Rows marked "!" ran with more than 1.0 core of foreign load and are upper bounds.
- The strict idle target (load under 0.5) needs the desktop session closed.

### 9.7 Trained on recorded Thought decisions (second run of 2026-11-15)

The path 9.6 listed as never tested: train the small models on what Thought really decided
instead of 13 solved examples. Measurement only, nothing is wired into the player. Code in
bench/decider/intuition, run by bench/decider/trained.sh, tables by intuition/report.py,
raw results in bench/decider/results/trained.

Data:

- Source: the 11 session transcripts the play relay wrote on 2026-11-14 and 2026-11-15
  (logs/brain/<run>/session-<n>.jsonl, all on the live game). The journal and the bot logs
  hold no decisions. intuition/extract.py makes one pair per decision point (session start
  and every wait_event result). The state is the event text, the last observe line Thought
  had seen, its last three decisions, the mode, and the active line of its project journal
  as it stood when the event arrived. The decision is what Thought did before its next
  wait_event: the goal it set, else say, arm, set_mode or inhibit, else wait.
- Real pairs: 470. By decision: place 126, craft 81, go_to 78, wait 73, mine 64, collect 27,
  say 5, smelt 5, set_mode 3, inhibit 2, give 2, attack 2, end 1, follow 1. A chat line
  drives 6 of the 470 (1.3 %) and the companion is visible in 31: this is solo building and
  crafting. The split-second calls this layer was meant for (fight or run, which target) do
  not occur once; the reflexes had handled them before Thought was asked.
- 204 of the 470 decisions (43 %) carry coordinates and 181 (39 %) an item and a count, which
  a classifier over goal names does not produce. Only wait (73, 16 %) is complete as a label.
- Split by time, whole sessions: train = 2026-11-14 (8 sessions, 362 pairs), test_real =
  2026-11-15 (3 sessions, 108 pairs, none with a chat line). The second day is another
  activity (tools, iron, armour: craft 44, mine 22 of 108) than the first (a hut and a dirt
  stair: place 118, go_to 72 of 362), so test_real measures transfer to a new day. Two
  cross-checks over all 470 stand next to it: leave one session out (11 fits) and 3 folds
  of whole sessions.
- test_hand = bench/decider/cases_nochat.jsonl: 48 cases in the real state format, none with
  a chat line, written and frozen before any training. 23 continue a project the way the
  recorded play does, 25 are driven by body, companion or danger (eating, sleeping,
  fighting, following, guarding, retreating, dusk). 14 of the 48 have a decision that never
  occurs in the training day.
- Synthetic pairs (intuition/synth.py), labelled synthetic, never in a test set. "vary":
  1086, three copies of every training pair with coordinates, ticks, ids, distances and
  stack counts varied and the recorded decision kept. "rules": 633, a grid over state fields
  for the decisions the recorded play lacks, labelled by the written playing rules. The
  rules set shares its author with test_hand and was written after it, so on test_hand its
  rows are an upper bound, as the keyword rule was in 9.2.
- Thought itself, timed from the same transcripts: median 1.7 s from an event to its next
  action (p90 3.2 s, 392 turns) and 1.2 s to decide to keep waiting (68 turns).

Results: correct decisions, with the 95 % interval in percent (Wilson). Every arm predicts
the goal name only. Latency is one decision at a time on 4 threads, the whole path from the
pair to a label, n = 468 per row (156 cases, 3 repetitions), behind bench/decider/idle.py.

| arm | trained on | test_real /108 | test_hand /48 | leave one session out /470 | p50 / p90 ms |
|---|---|---|---|---|---|
| uniform chance, 18 options | - | 6.0 | 2.7 | 26.1 | - |
| majority decision of the training day (place) | - | 8 (3.8 to 13.9) | 6 (5.9 to 24.7) | 126 (23.0 to 31.0) | - |
| repeat the last decision | - | 41 (29.4 to 47.4) | 9 (10.2 to 31.9) | 170 (32.0 to 40.6) | - |
| keyword rule of 9.2 | - | 4 (1.4 to 9.1) | 9 (10.2 to 31.9) | 16 (2.1 to 5.5) | under 1 |
| 135M as in 9.4: 12 solved examples cached, argmax | 12 examples | 8, calibrated 5 | 4, calibrated 1 | n/a | 469 / 739 |
| 1.5B 4-bit, same | 12 examples | 1, calibrated 15 | 5, calibrated 8 | n/a | 6627 / 8691 ! |
| encoder as in 9.4: nearest of 12 solved examples | 12 examples | 30 (20.2 to 36.9) | 8 (8.7 to 29.6) | n/a | 81 / 118 ! |
| a1 encoder, logistic head | 362 real | 53 (39.8 to 58.4) | 17 (23.4 to 49.6) | 151 (28.1 to 36.5) | 69 / 95 |
| a1 | real + vary + rules | 43 (31.1 to 49.2) | 30 (48.4 to 74.8) | n/a | 69 / 95 |
| a2 encoder, MLP head | 362 real | 54 (40.7 to 59.3) | 17 (23.4 to 49.6) | 159 (29.7 to 38.2) | 70 / 97 |
| a3 encoder and state features, logistic head | 362 real | 32 (21.8 to 38.8) | 12 (14.9 to 38.8) | 173 (32.6 to 41.3) | 72 / 96 |
| a4 encoder trained end to end, 6 epochs | 362 real | 61 (47.1 to 65.5) | 14 (18.2 to 43.2) | 3 folds: 234 (45.3 to 54.3) | as a1 |
| b1 encoder, nearest neighbour over the training set | 362 real | 50 (37.2 to 55.7) | 14 (18.2 to 43.2) | 131 (24.0 to 32.1) | 105 / 142 |
| b5 encoder, 5 nearest neighbours | 362 real | 55 (41.6 to 60.2) | 14 (18.2 to 43.2) | 168 (31.5 to 40.2) | 104 / 140 |
| c1 state features, logistic regression | 362 real | 37 (26.0 to 43.6) | 13 (16.6 to 41.0) | 184 (34.8 to 43.6) | 0.9 / 2.0 |
| c1 | real + vary + rules | 35 (24.3 to 41.7) | 28 (44.3 to 71.2) | n/a | 0.9 / 2.0 |
| c2 state features, gradient-boosted trees | 362 real | 39 (27.7 to 45.5) | 14 (18.2 to 43.2) | 149 (27.7 to 36.0) | 36 / 46 |
| c3 habit table (counts per event kind, no model) | 362 real | 49 (36.3 to 54.8) | 15 (19.9 to 45.3) | 183 (34.6 to 43.4) | not timed |
| d0 135M frozen, logistic head on its last state | 362 real | 29 (19.4 to 35.9) | 7 (7.2 to 27.2) | 121 (22.0 to 29.9) | 239 / 337 |
| d 135M with a low-rank adapter (top 8 layers, 8 epochs), argmax | 362 real | 46 (33.7 to 52.0) | 16 (21.7 to 47.5) | n/a | 239 / 337 |

- Conditions: no compiler ran during any latency row. The load target (under 0.5) was met in
  none: the 1-minute load read 1.7 to 5.2, the tail of the training jobs, and other
  processes used 0.8 of a core, as in 9.1. Rows marked "!" are upper bounds: the encoder
  pass alone ran with 1.4 cores of foreign load, and the 1.5B row was timed while my own
  training jobs used 5 cores (its clean figure for a suffix of this length is 2804 / 2975
  ms, 9.4). The b rows were timed against 2081 stored rows. The d rows are the adapter
  prompt (84 tokens, nothing cached) through llama.cpp on the base weights: an adapter
  merged into the weights changes no shapes, so the cost is the same; d's accuracy comes
  from the training framework.
- Two launches died with an illegal-instruction signal before computing anything (a timing
  probe and the first d0 run), as one did in 9.1; both ran on the second try. Cause unknown.
- The 3-fold cross-check (whole sessions, 470 decisions): a1 158, a2 179, b5 168, c1 203,
  c2 162, c3 178, repeat-last 170, a4 234 (45.3 to 54.3).
- (a) against (c). On test_real the frozen encoder (a1) is right where c1 is wrong 30 times
  and the reverse 14 times (exact sign test p = 0.023), against c2 26 and 12 (p = 0.034),
  against the habit table 14 and 10 (p = 0.54). The encoder trained end to end (a4) leads c1
  by 35 to 11 (p = 0.0005), c2 by 32 to 10 (p = 0.0009) and the habit table by 21 to 9 (p =
  0.043). Over all sessions the order of the frozen encoder reverses: leave one session out
  gives a1 151, c1 184, c3 183; 3 folds give a1 158, c1 203. The encoder trained end to end
  (a4) keeps its lead on the 3 folds: 234 of 470 against 203 for c1 (right where c1 is wrong
  85 times, the reverse 54, p = 0.011) and against every other arm (p under 0.001). By fold
  it reads 37, 70, 127 against c1's 40, 73, 90: level on two folds and far ahead on the one
  that holds the long building sessions. So (a) is clearly above (c) only when the encoder
  itself is trained; with a frozen encoder the feature classifier is the better of the two.
- No arm is right in more than 61 of 108 decisions, and on the cross-checks none passes 50 %.
  "Repeat the last decision" gets 41 of 108 and 170 of 470 with no model at all.
- Yesterday's variants on real decisions: the keyword rule falls from 63/65 to 4/108, since
  it read chat lines and real decisions have none. The 135M model with cached examples
  falls from 37/65 to 8/108 (it answers go_to 84 times) and its suffix now costs 469 ms
  behind a 1450-token cached prefix. The 1.5B model falls from 54/65 to 1/108 (it answers
  arm 99 times; 15 with calibration). The encoder with the nearest of 12 examples gets 30;
  the same encoder with a trained head 53. Training on recorded decisions is worth 23 to 31
  cases of 108 over solved examples.
- Synthetic "vary" changes nothing (a1 53 to 53, a2 54 to 51): the fields it varies are the
  ones the text rendering drops. Synthetic "rules" lifts test_hand from 17 to 30 of 48 and
  costs test_real 10 cases (53 to 43): its states reuse real event kinds with other
  decisions.
- The plan line of the journal in the text (results/trained/classic-plan.json) lifts leave
  one session out from 151 to 203 of 470 (a1) and from 159 to 208 (a2) and lowers test_real
  from 53 to 41 and from 54 to 51: it helps while the project is one the model has seen and
  misleads on a new day.
- Goal plus item as the decision ("craft stick", "mine iron_ore") instead of the goal name:
  best arm 28 of 108.

Accuracy against coverage on test_real, for the rule "act when confidence is at least t,
else wake Thought". Cells are "decisions taken: percent of those that match Thought". The
last two columns are the number taken at 95 % precision or better, first with t chosen on
the test day itself (an upper bound), then with t chosen on the training day (leave one
training session out) and applied unchanged.

| arm | t = 0.5 | 0.6 | 0.7 | 0.8 | 0.9 | 0.95 | at 95 %, t from the test day | at 95 %, t from the training day |
|---|---|---|---|---|---|---|---|---|
| a1 | 36: 69.4 | 16: 93.8 | 4: 100 | 0 | 0 | 0 | 11 (10.2 %) | 4 (3.7 %), all 4 right |
| a2 | 63: 61.9 | 42: 66.7 | 26: 76.9 | 6: 100 | 1: 100 | 0 | 12 (11.1 %) | 4 (3.7 %), all 4 right |
| a4 | 61: 67.2 | 39: 74.4 | 22: 95.5 | 14: 100 | 4: 100 | 0 | 22 (20.4 %) | not measured |
| b5 | 82: 58.5 | 42: 69.0 | 42: 69.0 | 13: 69.2 | 13: 69.2 | 9: 55.6 | 4 (3.7 %) | 0, no such t exists |
| c1 | 43: 46.5 | 25: 48.0 | 13: 61.5 | 8: 62.5 | 2: 0 | 0 | 0 | 0 |
| c2 | 73: 45.2 | 43: 51.2 | 26: 61.5 | 17: 58.8 | 10: 60.0 | 3: 33.3 | 1 (0.9 %) | 0, no such t exists |
| c3 | 86: 55.8 | 20: 75.0 | 11: 100 | 10: 100 | 10: 100 | 0 | 12 (11.1 %) | 0, no such t exists |
| d | 54: 59.3 | 30: 60.0 | 10: 60.0 | 3: 100.0 | 1: 100.0 | 1: 100.0 | 5 (4.6 %) | not measured |

- Coverage at 95 % precision: at best 22 of 108 decisions (20.4 %, a4) with the threshold
  chosen on the test day, and 4 of 108 (3.7 %) with a threshold fixed beforehand. On the
  cross-checks, thresholds chosen with hindsight: leave one session out, at best 14 of 470
  (3.0 %, c2); 3 folds, at best 21 of 470 (4.5 %, c1), and 3 of 470 (0.6 %) for a4.
- What the confident predictions are: a4's 22 are 11 wait, 8 craft, 3 mine (one wrong); a1's
  16 above 0.6 are 9 wait and 7 craft; the habit table's 11 above 0.7 are all wait after a
  goal_blocked event. A confident "craft" still lacks the item and the count, so the part a
  model could really take over is wait: the body reports "path problem, re-planning" and
  Thought answers by waiting.
- As a wake filter (wait against act, intuition/wake_filter.py, 18 waits in test_real): the
  one-line rule "goal_blocked means wait" filters 11 of the 108 test decisions, all 11
  rightly, and 41 of the 362 training decisions, 35 rightly (85 %). With a threshold from
  the training day a1 filters 5 of 108 (all right), c1 and c2 none; with the threshold
  chosen on the test day a1 filters 14, c1 12, c2 11, the rule 11, all at 100 %.
- Filtering every recorded wait would save 98 s of the 896 s Thought spent between an event
  and its next call (11 %) and makes no action faster.

Verdict against the bar (median under 500 ms; on held-out cases clearly above the
non-neural baseline, with accuracy that can be acted on):

- The comparison half of the bar (clearly above the non-neural baseline) is met by one arm,
  a4, on both views. The accuracy half is met by none.
- Latency: passes for every trained arm. 0.9 ms for the feature classifier, 69 to 105 ms
  for the encoder arms, 239 ms for the 135M model with the short adapter prompt.
- Accuracy: does not pass for any arm. The best is right in 61 of 108 decisions on a new day
  and 234 of 470 on the 3 folds, the takeover at 95 % precision is 4 of 108 with a threshold
  fixed beforehand (22 with hindsight, at most 4.5 % on the cross-checks), and what is taken
  is the goal name without its item, count or coordinates. Intuition cannot be filled with a
  model trained on these 470 decisions, on this hardware or on a faster one.
- (a) encoder with a trained head. a1 and a2 (frozen encoder): lost a comparison, to the
  bar and to the feature classifier on both cross-checks (151 and 159 on leave one session
  out against 184; 158 and 179 on the 3 folds against 203). a4, the encoder trained end to
  end, works as a predictor of the goal name and is the best arm on both views (61 of 108,
  234 of 470, clearly above c1 on both). It lost the comparison to the bar: it matches
  Thought in every second decision, and at 95 % precision it takes 3 of 470 decisions on
  the folds (0.6 %; above a confidence of 0.9 it takes 51, of which 78 % match).
- (b) encoder nearest neighbour over the training set: lost a comparison, to the bar and to
  c1 on the cross-checks (131 and 168 against 184); level with a1 on test_real (50 and 55
  against 53).
- (c) feature classifier and habit table: lost a comparison, to the bar (32 to 45 % right).
  c1 is the best arm without a trained network over all sessions (184, 203) and the habit
  table is level with the frozen encoder on the new day (49 against 53, p = 0.54). Neither
  is the better Intuition: a4 is ahead of both. c1 stays as the floor to beat: 0.9 ms, no
  model file, readable.
- (d) 135M with a low-rank adapter: lost a comparison, to a4 (46 against 61 of 108, right
  alone 7 times against 22, p = 0.008) and not separated from c1 (right alone 23 times
  against 14, p = 0.19), at 3.5 times the encoder's latency and 64 minutes of training
  against 17. Not closed: the adapter covered the top 8 of 30 layers for 8 epochs and its
  loss was still falling (0.99 at the end).
- (d0) frozen 135M with a trained head: lost a comparison, to the 33M encoder with the
  same head (29 against 53) at 3.5 times its latency.
- Untrained variants of 9.4 on real decisions: lost a comparison, to chance in the case of
  the 135M model (8 against 6.0) and to "repeat the last decision" in the case of the
  nearest-example encoder (30 against 41).
- Synthetic variation of real pairs: closed under its premise. The varied fields
  (coordinates, ticks, ids) are not in the model input, so the copies are near duplicates.
  Reopen only with a state rendering that keeps positions.
- Wake filter: works, as a rule and without a model. "goal_blocked means wait" was right in
  46 of 52 recorded cases (35 of 41, then 11 of 11). It belongs in the body's event
  reporting, not in this layer: such an event need not wake Thought at all.
- Nothing else is proven closed. Every other negative above is a lost comparison on 470
  decisions from two days of solo play.

Why: once the model is trained, this machine is no longer the limit, since a 33M encoder
with a head decides in 69 ms. The limit is what the state contains. After the same kind of
event Thought went different ways (after a finished craft: craft 20, mine 13, place 5,
wait 4 of 42 cases), because the next step comes from its plan and its inventory, and
neither is in the one-line state. A model trained on these pairs learns to
be a weaker Thought at the job Thought exists for. It does not learn the fast judgement
between reflex and thought this layer was meant for, because Thought was never asked for
that judgement in the recorded play.

### 9.8 Open after the second run

- A decision log written by the body at the moment Thought decides: the fresh snapshot (the
  pairs above carry the last observe line Thought saw, often thousands of ticks old), the
  inventory, the running plan step, and the full call with its parameters. Without the
  inventory and the plan the next craft or mine step cannot be predicted by anything.
  Built the same day (section 8, "Decision log"); no model has been trained on it yet.
- The wake filter of 9.7 is in the body's event reporting now (section 8, "Wakes on a
  blocked goal").
- The fast judgements themselves as the target (mode, gains, arming, inhibiting, fight or
  run): 10 of the 470 recorded decisions. They need play with the companion and with danger,
  which the two recorded days do not contain. Never tested.
- a4 with a threshold fixed on the training day, a4 on leave one session out, and d on any
  cross-check: not measured (one a4 fit takes 17 to 30 minutes on this CPU next to other
  jobs, one d fit 64).
- d with more than the top 8 of 30 layers adapted and more epochs: with all layers adapted,
  one epoch over 362 pairs did not finish in 11 minutes here. A first d run (5 epochs, half
  the learning rate, loss 1.50) was lost to a scoring fault before it was scored.
- a4 and d trained with the synthetic "rules" pairs (phases encoder-ft-rules and lora_rules
  of trained.sh): not run, for CPU time.
- test_hand is 48 cases by one author; its interval is 25 points wide.

## 10. References

Neuroscience and psychology
- [B91] Brown et al. 1991, New observations on the normal auditory startle reflex in man, Brain 114:1891. https://scilit.com/publications/14fcaaca435603b42b8cfcb68c539497
- [BWF98] Blakemore, Wolpert, Frith 1998, Central cancellation of self-produced tickle sensation. https://doi.org/10.1038/2870
- [C11] Carlsen et al. 2011, Considerations for the use of a startling acoustic stimulus in studies of motor preparation. https://doi.org/10.1016/j.neubiorev.2010.04.009
- [Ca71] Cabanac 1971, Physiological role of pleasure. https://doi.org/10.1126/science.173.4002.1103
- [Cr02] Craig 2002, How do you feel? Interoception. https://doi.org/10.1038/nrn894
- [CS00] Collewijn, Smeets 2000, Early components of the human vestibulo-ocular response. https://doi.org/10.1152/jn.2000.84.1.376
- [D98] Dimitrijevic, Gerasimenko, Pinter 1998, Evidence for a spinal central pattern generator in humans. https://doi.org/10.1111/j.1749-6632.1998.tb09062.x
- [DND05] Daw, Niv, Dayan 2005, Uncertainty-based competition between prefrontal and dorsolateral striatal systems. https://doi.org/10.1038/nn1560
- [FB83] Fischer, Boch 1983, Saccadic eye movements after extremely short reaction times in the monkey. https://doi.org/10.1016/0006-8993(83)90760-6 (human express-saccade ranges from Fischer and Ramsperger 1984, via review)
- [FL88] Fanselow, Lester 1988, A functional behavioristic approach to aversively motivated behavior: predatory imminence. https://doi.org/10.4324/9780203781647-14
- [G85] Grillner 1985, Neurobiological bases of rhythmic motor acts in vertebrates. https://doi.org/10.1126/science.3975635
- [Gm75] Graham 1975, The more or less startling effects of weak prestimulation. https://doi.org/10.1111/j.1469-8986.1975.tb01284.x
- [Go99] Gollwitzer 1999, Implementation intentions. https://doi.org/10.1037/0003-066x.54.7.493
- [Gr98] Graybiel 1998, The basal ganglia and chunking of action repertoires. https://doi.org/10.1006/nlme.1998.3843
- [GT70] Groves, Thompson 1970, Habituation: a dual-process theory. https://doi.org/10.1037/h0029810
- [J99] Jog et al. 1999, Building neural representations of habits. https://doi.org/10.1126/science.286.5445.1745
- [K15] Kurtzer 2015, Long-latency reflexes account for limb biomechanics through several supraspinal pathways. https://doi.org/10.3389/fnint.2014.00099
- [KG14] Keramati, Gutkin 2014, Homeostatic reinforcement learning. https://doi.org/10.7554/eLife.04811
- [Kl00] Klein 2000, Inhibition of return. https://doi.org/10.1016/S1364-6613(00)01452-2
- [KLZ13] Krauzlis, Lovejoy, Zenon 2013, Superior colliculus and visual spatial attention. https://doi.org/10.1146/annurev-neuro-062012-170249
- [L76] Lee 1976, A theory of visual control of braking based on time-to-collision. https://doi.org/10.1068/p050437
- [M07] Mobbs et al. 2007, When fear is near. https://doi.org/10.1126/science.1144298
- [Ma18] Martiros, Burgess, Graybiel 2018, striatal neurons delimit behavioural sequences. https://doi.org/10.1016/j.cub.2018.01.031
- [Mi96] Mink 1996, The basal ganglia: focused selection and inhibition of competing motor programs. https://doi.org/10.1016/s0301-0082(96)00042-1
- [N81] Norman 1981, Categorization of action slips. https://doi.org/10.1037/0033-295X.88.1.1
- [P12] Pruszynski, Scott 2012, Optimal feedback control and the long-latency stretch response. https://doi.org/10.1007/s00221-012-3041-8
- [P14] Pruszynski 2014, Primary motor cortex and fast feedback responses to mechanical perturbations: a primer. https://doi.org/10.3389/fnint.2014.00072
- [RPG99] Redgrave, Prescott, Gurney 1999, The basal ganglia: a vertebrate solution to the selection problem? https://doi.org/10.1016/s0306-4522(98)00319-4
- [So63] Sokolov 1963, Higher nervous functions: the orienting reflex. https://doi.org/10.1146/annurev.ph.25.030163.002553
- [SB75] Scaife, Bruner 1975, The capacity for joint visual attention in the infant (basis for O4). https://doi.org/10.1038/253265a0
- [TS66] Thompson, Spencer 1966, Habituation: a model phenomenon. https://doi.org/10.1037/h0022681
- [VL08] Verbruggen, Logan 2008, Response inhibition in the stop-signal paradigm. https://doi.org/10.1016/j.tics.2008.07.005
- [VS99] Valls-Sole et al. 1999, Patterned ballistic movements triggered by a startle in healthy humans. https://doi.org/10.1111/j.1469-7793.1999.0931u.x
- [WMK98] Wolpert, Miall, Kawato 1998, Internal models in the cerebellum. https://doi.org/10.1016/s1364-6613(98)01221-2
- [YM13] Yilmaz, Meister 2013, Rapid innate defensive responses of mice to looming visual stimuli. https://doi.org/10.1016/j.cub.2013.08.015

Robotics and agents
- [Bo97] Bonasso et al. 1997, Experiences with an architecture for intelligent, reactive agents (3T). https://doi.org/10.1080/095281397147103
- [Br86] Brooks 1986, A robust layered control system for a mobile robot. https://doi.org/10.1109/jra.1986.1087032
- [CO18] Colledanchise, Ogren 2018, Behavior Trees in Robotics and AI. https://doi.org/10.1201/9780429489105
- [HR25] Shi et al. 2025, Hi Robot: open-ended instruction following with hierarchical VLA models. https://arxiv.org/abs/2502.19417
- [Hx25] Figure AI 2025, Helix. https://figure.ai/news/helix
- [L25] Li et al. 2025, Parallelized planning-acting for efficient LLM-based multi-agent systems in Minecraft. https://arxiv.org/abs/2503.03505
- [MC25] White et al. 2025, Collaborating action by action: a multi-agent LLM framework for embodied reasoning (Mindcraft). https://arxiv.org/abs/2504.17950
- [PMG06] Prescott et al. 2006, A robot model of the basal ganglia. https://doi.org/10.1016/j.neunet.2005.06.049
- [PS24] Altera 2024, Project Sid: many-agent simulations toward AI civilization (PIANO). https://arxiv.org/abs/2411.00114
- [SPS99] Sutton, Precup, Singh 1999, Between MDPs and semi-MDPs (options). https://doi.org/10.1016/s0004-3702(99)00052-1
- [SS23] Lin et al. 2023, SwiftSage. https://doi.org/10.52202/075280-1034
- [TR24] Christakopoulou, Mourad, Mataric 2024, Agents thinking fast and slow: a Talker-Reasoner architecture. https://arxiv.org/abs/2410.08328
- [V23] Wang et al. 2023, Voyager. https://arxiv.org/abs/2305.16291
- AuRA (Arkin; planner selects and configures motor schemas): summarised from https://sites.cc.gatech.edu/ai/robot-lab/online-publications/ISRMA94.pdf and https://cdn.aaai.org/FLAIRS/2003/Flairs03-005.pdf

Minecraft software
- [BAR] Baritone settings. https://github.com/cabaletta/baritone/blob/master/src/api/java/baritone/api/Settings.java
- [MC-modes] Mindcraft modes and setMode. https://github.com/kolbytn/mindcraft/blob/main/src/agent/modes.js and https://github.com/kolbytn/mindcraft/blob/main/src/agent/commands/actions.js
- [MET] Meteor client NoFall and AutoTotem. https://github.com/MeteorDevelopment/meteor-client
- [MF-AE] mineflayer-auto-eat 5.0.3. https://github.com/linkle69/mineflayer-auto-eat
- [MF-AM] mineflayer-armor-manager. https://github.com/PrismarineJS/MineflayerArmorManager
- [MF-PF] mineflayer-pathfinder. https://github.com/PrismarineJS/mineflayer-pathfinder
- [MF-PVP] mineflayer-pvp. https://github.com/PrismarineJS/mineflayer-pvp
- [MF-SM] mineflayer-statemachine. https://github.com/PrismarineJS/mineflayer-statemachine

Minecraft mechanics (minecraft.wiki)
- [W-Attr] https://minecraft.wiki/w/Attribute
- [W-Cr] https://minecraft.wiki/w/Creeper
- [W-Crk] https://minecraft.wiki/w/Creaking
- [W-Dmg] https://minecraft.wiki/w/Damage
- [W-End] https://minecraft.wiki/w/Enderman
- [W-Hun] https://minecraft.wiki/w/Hunger
- [W-Ph] https://minecraft.wiki/w/Phantom
- [W-Sh] https://minecraft.wiki/w/Shield
- [W-Sk] https://minecraft.wiki/w/Skeleton
- [W-Spr] https://minecraft.wiki/w/Sprinting
- [W-Tot] https://minecraft.wiki/w/Totem_of_Undying
