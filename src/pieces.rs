//! The four piece kinds, and the glue that writes them into the world.
//!
//! Each piece is the same `StructurePiece` the engine runs for a vanilla structure piece: it answers
//! its bounds, places its blocks into one chunk at a time (`post_process`), and saves its own fields
//! beside the engine's `BB`/`O`/`GD` (`add_additional_save_data`), which [`crate::support`]'s
//! `load_piece` reads back. The layout itself — which pieces, where, and how many — is
//! [`crate::runtime`]'s.
//!
//! The four kinds and what each writes (all from the upstream classes named in each function):
//!
//! | Kind | Box | Writes |
//! |---|---|---|
//! | `Entrance` | 5×(surface−floor−1)×5 | patched+randomized shaft walls, plank floor, carved 3×3 shaft, ladder, stone-brick doorway, four legs |
//! | `Corridor` | 9×8×24 | randomized walls, plank/randomized floor, vaulted carve, fence supports on the covered cells, rails, hanging lanterns, gravel piles, vines |
//! | `Branch` | 5×5×8 | the same at small scale, with a torch-free, support-and-rail run |
//! | `Stairs` | 5×9×5 | five descending slices, each with its floor and a rail that climbs the passage |
//! | `Room` | 10×5×5 | brick floor/walls/ceiling patch, iron-bar columns, crafting table, the doorway into the corridor |
//!
//! What is deliberately absent is in `notes.md`: turns, layered intersections, ore deposits,
//! zombie-villager rooms, the terrain-driven surface tunnel, minecarts, chests/furnaces and the
//! biome-specific decorations (`addBiomeDecorations` needs a configured-feature registry and a
//! biome, neither of which a `PieceContext` exposes).

use std::sync::Arc;

use mod_api::{
    self, BlockMetadata, BlockState, BoundingBox, Compound, PieceContext, StructureError,
    StructurePiece, Tag,
};

use crate::runtime::{
    self, BlockRef, Box3, Facing, LegVariant, Opening, Palette, PieceKind, PieceSpec, Randomizer,
    Weighted,
};

/// How far a leg reaches down before the port stops looking for ground.
pub const MAX_LEG_DEPTH: i32 = 6;

/// The blocks the port always names itself; the palette's own states come from the config.
const ALWAYS: [(&str, &[(&str, &str)]); 7] = [
    ("minecraft:air", &[]),
    ("minecraft:cobweb", &[]),
    ("minecraft:rail", &[]),
    ("minecraft:powered_rail", &[("powered", "true")]),
    ("minecraft:lantern", &[("hanging", "true")]),
    ("minecraft:iron_bars", &[]),
    ("minecraft:crafting_table", &[]),
];

/// The blocks the ore deposit and the zombie villager room place, resolved once.
#[derive(Debug)]
pub struct Extra {
    pub cobblestone: BlockState,
    /// `OreDeposit.OreType`'s blocks, in [`runtime::ORE_WEIGHTS`] order.
    pub ores: [BlockState; 8],
    pub stone: BlockState,
    pub stone_bricks: BlockState,
    pub stone_slab: BlockState,
    pub cobblestone_slab: BlockState,
    pub top_stone_slab: BlockState,
    pub cobblestone_wall: BlockState,
    pub redstone_torch: BlockState,
    pub stone_button: BlockState,
    pub anvil: BlockState,
    pub smithing_table: BlockState,
    pub blast_furnace: BlockState,
    pub spawner: BlockState,
    pub barrel: BlockState,
    pub iron_bars: BlockState,
}

fn resolve_extra(metadata: &dyn BlockMetadata) -> Result<Extra, StructureError> {
    let s = |name: &str, properties: &[(&str, &str)]| {
        resolve_state(metadata, &BlockRef::with(name, properties))
    };
    let ore = |name: &str| s(name, &[]);
    Ok(Extra {
        cobblestone: s("minecraft:cobblestone", &[])?,
        ores: [
            s("minecraft:cobblestone", &[])?,
            ore("minecraft:coal_ore")?,
            ore("minecraft:iron_ore")?,
            ore("minecraft:redstone_ore")?,
            ore("minecraft:gold_ore")?,
            ore("minecraft:lapis_ore")?,
            ore("minecraft:emerald_ore")?,
            ore("minecraft:diamond_ore")?,
        ],
        stone: s("minecraft:stone", &[])?,
        stone_bricks: s("minecraft:stone_bricks", &[])?,
        stone_slab: s("minecraft:stone_slab", &[])?,
        cobblestone_slab: s("minecraft:cobblestone_slab", &[])?,
        top_stone_slab: s("minecraft:stone_slab", &[("type", "top")])?,
        cobblestone_wall: s("minecraft:cobblestone_wall", &[])?,
        redstone_torch: s("minecraft:redstone_torch", &[])?,
        stone_button: s("minecraft:stone_button", &[])?,
        anvil: s("minecraft:anvil", &[])?,
        smithing_table: s("minecraft:smithing_table", &[])?,
        blast_furnace: s("minecraft:blast_furnace", &[])?,
        spawner: s("minecraft:spawner", &[])?,
        barrel: s("minecraft:barrel", &[])?,
        iron_bars: s("minecraft:iron_bars", &[])?,
    })
}

/// One randomizer with its states resolved for this run.
#[derive(Debug)]
pub struct ResolvedRandomizer {
    default: BlockState,
    entries: Vec<(BlockState, f32)>,
}

impl ResolvedRandomizer {
    /// The state a `nextFloat` draw picks, by cumulative probability, else the default.
    pub fn pick(&self, roll: f32) -> BlockState {
        let mut cumulative = 0.0;
        for (state, probability) in &self.entries {
            cumulative += probability;
            if roll < cumulative {
                return *state;
            }
        }
        self.default
    }
}

/// A palette's states resolved against the running game's block metadata.
#[derive(Debug)]
pub struct Resolved {
    pub main: BlockState,
    pub support: BlockState,
    pub slab: BlockState,
    pub gravel: BlockState,
    pub stone_wall: BlockState,
    pub stone_slab: BlockState,
    pub trapdoor: BlockState,
    pub small_leg: BlockState,
    pub main_random: ResolvedRandomizer,
    pub floor_random: ResolvedRandomizer,
    pub brick_random: ResolvedRandomizer,
    pub leg_random: ResolvedRandomizer,
    pub air: BlockState,
    pub cobweb: BlockState,
    pub rail: BlockState,
    pub powered_rail: BlockState,
    pub lantern: BlockState,
    pub iron_bars: BlockState,
    pub crafting_table: BlockState,
    pub ladders: [BlockState; 4],
    pub vines: [BlockState; 4],
    /// The flat rails a stair's last step carries: `[north_south, east_west]`.
    pub flat_rails: [BlockState; 2],
    /// The ascending rail for each [`LADDER_FACINGS`] entry, rising toward that facing's opposite.
    pub ascending_rails: [BlockState; 4],
    /// The powered ascending rail for the same four directions.
    pub powered_ascending_rails: [BlockState; 4],
    pub extra: Extra,
}

/// The four ladder states, in [`Facing`] order.
const LADDER_FACINGS: [Facing; 4] = [Facing::South, Facing::West, Facing::North, Facing::East];

impl Resolved {
    pub fn ladder(&self, facing: Facing) -> BlockState {
        self.ladders[facing_index(facing)]
    }

    pub fn vine(&self, facing: Facing) -> BlockState {
        self.vines[facing_index(facing)]
    }

    /// The rail a stair step carries for a piece running into `facing`: the last step is flat along
    /// the run, every other step rises toward the piece's entrance.
    pub fn stair_rail(&self, facing: Facing, last_step: bool) -> BlockState {
        if last_step {
            if facing.axis() == 2 {
                self.flat_rails[0]
            } else {
                self.flat_rails[1]
            }
        } else {
            self.ascending_rails[facing_index(facing)]
        }
    }

    /// The powered rail the first step carries, in the same direction.
    pub fn powered_stair_rail(&self, facing: Facing) -> BlockState {
        self.powered_ascending_rails[facing_index(facing)]
    }
}

fn facing_index(facing: Facing) -> usize {
    LADDER_FACINGS
        .iter()
        .position(|candidate| *candidate == facing)
        .unwrap_or(0)
}

/// Resolve a palette against `metadata`; an unknown block or property is a typed error, never a
/// panic.
pub fn resolve(
    metadata: &dyn BlockMetadata,
    palette: &Palette,
) -> Result<Resolved, StructureError> {
    let state = |reference: &BlockRef| resolve_state(metadata, reference);
    let mut always = Vec::new();
    for (name, properties) in ALWAYS {
        always.push(state(&BlockRef::with(name, properties))?);
    }
    let mut ladders = Vec::new();
    let mut vines = Vec::new();
    let mut ascending_rails = Vec::new();
    let mut powered_ascending_rails = Vec::new();
    for facing in LADDER_FACINGS {
        ladders.push(state(&BlockRef::with(
            "minecraft:ladder",
            &[("facing", facing.name())],
        ))?);
        vines.push(state(&BlockRef::with(
            "minecraft:vine",
            &[(facing.name(), "true")],
        ))?);
        let shape = runtime::stair_rail_shape(facing, false);
        ascending_rails.push(state(&BlockRef::with(
            "minecraft:rail",
            &[("shape", shape)],
        ))?);
        powered_ascending_rails.push(state(&BlockRef::with(
            "minecraft:powered_rail",
            &[("powered", "true"), ("shape", shape)],
        ))?);
    }
    let flat_rails = [
        state(&BlockRef::with(
            "minecraft:rail",
            &[("shape", "north_south")],
        ))?,
        state(&BlockRef::with("minecraft:rail", &[("shape", "east_west")]))?,
    ];
    let [
        air,
        cobweb,
        rail,
        powered_rail,
        lantern,
        iron_bars,
        crafting_table,
    ] = <[BlockState; 7]>::try_from(always).map_err(|_| {
        StructureError("bettermineshafts: the fixed block table is the wrong length".into())
    })?;
    let extra = resolve_extra(metadata)?;
    Ok(Resolved {
        extra,
        main: state(&palette.blocks.main)?,
        support: state(&palette.blocks.support)?,
        slab: state(&palette.blocks.slab)?,
        gravel: state(&palette.blocks.gravel)?,
        stone_wall: state(&palette.blocks.stone_wall)?,
        stone_slab: state(&palette.blocks.stone_slab)?,
        trapdoor: state(&palette.blocks.trapdoor)?,
        small_leg: state(&palette.blocks.small_leg)?,
        main_random: resolve_randomizer(metadata, &palette.randomizers.main)?,
        floor_random: resolve_randomizer(metadata, &palette.randomizers.floor)?,
        brick_random: resolve_randomizer(metadata, &palette.randomizers.brick)?,
        leg_random: resolve_randomizer(metadata, &palette.randomizers.leg)?,
        air,
        cobweb,
        rail,
        powered_rail,
        lantern,
        iron_bars,
        crafting_table,
        ladders: <[BlockState; 4]>::try_from(ladders).map_err(|_| {
            StructureError("bettermineshafts: the ladder table is the wrong length".into())
        })?,
        vines: <[BlockState; 4]>::try_from(vines).map_err(|_| {
            StructureError("bettermineshafts: the vine table is the wrong length".into())
        })?,
        flat_rails,
        ascending_rails: <[BlockState; 4]>::try_from(ascending_rails).map_err(|_| {
            StructureError("bettermineshafts: the ascending rail table is the wrong length".into())
        })?,
        powered_ascending_rails: <[BlockState; 4]>::try_from(powered_ascending_rails).map_err(
            |_| {
                StructureError(
                    "bettermineshafts: the powered rail table is the wrong length".into(),
                )
            },
        )?,
    })
}

fn resolve_state(
    metadata: &dyn BlockMetadata,
    reference: &BlockRef,
) -> Result<BlockState, StructureError> {
    let block = crate::support::key_for(&reference.name).ok_or_else(|| {
        StructureError(format!(
            "bettermineshafts: the palette names block {}, which the mod did not request",
            reference.name
        ))
    })?;
    let constraints: Vec<mod_api::StateConstraint> = reference
        .properties
        .iter()
        .map(|(name, value)| mod_api::StateConstraint::new(name.as_str(), value.as_str()))
        .collect();
    BlockState::of(metadata, block, &constraints).map_err(|error| {
        StructureError(format!(
            "bettermineshafts: {} {:?} is not a state: {error:?}",
            reference.name, reference.properties
        ))
    })
}

fn resolve_randomizer(
    metadata: &dyn BlockMetadata,
    randomizer: &Randomizer,
) -> Result<ResolvedRandomizer, StructureError> {
    let mut entries = Vec::new();
    for entry in &randomizer.entries {
        entries.push((resolve_state(metadata, &entry.state)?, entry.probability));
    }
    Ok(ResolvedRandomizer {
        default: resolve_state(metadata, &randomizer.default)?,
        entries,
    })
}

/// One piece of a mineshaft, ready to place.
#[derive(Debug)]
pub struct Piece {
    pub spec: PieceSpec,
    pub palette: Palette,
    pub blocks: Arc<Resolved>,
}

impl Piece {
    pub fn new(spec: PieceSpec, palette: Palette, blocks: Arc<Resolved>) -> Self {
        Self {
            spec,
            palette,
            blocks,
        }
    }

    fn set(&self, ctx: &mut dyn PieceContext, local: [i32; 3], state: BlockState) {
        let _ = ctx.set_block(self.spec.local_to_world(local), state);
    }

    fn get(&self, ctx: &mut dyn PieceContext, local: [i32; 3]) -> Option<BlockState> {
        ctx.block(self.spec.local_to_world(local))
    }

    fn roll(&self, ctx: &mut dyn PieceContext) -> f32 {
        ctx.random().next_float()
    }

    fn pick(&self, ctx: &mut dyn PieceContext, randomizer: &ResolvedRandomizer) -> BlockState {
        let roll = ctx.random().next_float();
        randomizer.pick(roll)
    }

    /// Whether a local cell is inside the chunk this pass writes; outside cells are dropped by
    /// `set_block` anyway, so reading them first would only waste a noise-column read.
    fn visible(&self, ctx: &dyn PieceContext, local: [i32; 3]) -> bool {
        ctx.chunk_box().is_inside(self.spec.local_to_world(local))
    }

    fn is_air_mut(&self, ctx: &mut dyn PieceContext, local: [i32; 3]) -> bool {
        match self.get(ctx, local) {
            Some(state) => ctx.is_air(state),
            None => true,
        }
    }

    /// Write `state` only where the cell currently holds air.
    fn set_if_air(&self, ctx: &mut dyn PieceContext, local: [i32; 3], state: BlockState) {
        if !self.visible(ctx, local) {
            return;
        }
        if !self.is_air_mut(ctx, local) {
            return;
        }
        self.set(ctx, local, state);
    }

    /// Randomize the terrain a piece cuts through, and patch air with the main block: upstream's
    /// `chanceReplaceNonAir(replacementRate, …)` then `replaceAirOrChains(mainBlockState)`.
    fn patch_shell(&self, ctx: &mut dyn PieceContext, size: [i32; 3]) {
        for y in 0..size[1] {
            for x in 0..size[0] {
                for z in 0..size[2] {
                    let local = [x, y, z];
                    if !self.visible(ctx, local) {
                        continue;
                    }
                    match self.get(ctx, local) {
                        Some(state) if !ctx.is_air(state) => {
                            if self.roll(ctx) < self.palette.replacement_rate {
                                let picked = self.pick(ctx, &self.blocks.main_random);
                                self.set(ctx, local, picked);
                            }
                        }
                        _ => self.set(ctx, local, self.blocks.main),
                    }
                }
            }
        }
    }

    /// Randomize the terrain a piece cuts through without patching its air: upstream's
    /// `chanceReplaceNonAir` alone, from `from_y` to the box's top.
    fn randomize_non_air(&self, ctx: &mut dyn PieceContext, size: [i32; 3], from_y: i32) {
        for y in from_y..size[1] {
            for x in 0..size[0] {
                for z in 0..size[2] {
                    let local = [x, y, z];
                    if !self.visible(ctx, local) {
                        continue;
                    }
                    if let Some(state) = self.get(ctx, local)
                        && !ctx.is_air(state)
                        && self.roll(ctx) < self.palette.replacement_rate
                    {
                        let picked = self.pick(ctx, &self.blocks.main_random);
                        self.set(ctx, local, picked);
                    }
                }
            }
        }
    }

    fn carve(&self, ctx: &mut dyn PieceContext, from: [i32; 3], to: [i32; 3]) {
        for y in from[1]..=to[1] {
            for x in from[0]..=to[0] {
                for z in from[2]..=to[2] {
                    self.set(ctx, [x, y, z], self.blocks.air);
                }
            }
        }
    }

    fn fill(&self, ctx: &mut dyn PieceContext, from: [i32; 3], to: [i32; 3], state: BlockState) {
        for y in from[1]..=to[1] {
            for x in from[0]..=to[0] {
                for z in from[2]..=to[2] {
                    self.set(ctx, [x, y, z], state);
                }
            }
        }
    }

    /// `BetterMineshaftPiece.generateLeg`: extend a post down while it is air, bounded.
    fn leg(&self, ctx: &mut dyn PieceContext, x: i32, z: i32) {
        let mut y = -1;
        for _ in 0..MAX_LEG_DEPTH {
            let local = [x, y, z];
            if !self.visible(ctx, local) {
                return;
            }
            if !self.is_air_mut(ctx, local) {
                return;
            }
            let picked = self.pick(ctx, &self.blocks.leg_random);
            self.set(ctx, local, picked);
            y -= 1;
        }
    }

    fn write_entrance(&self, ctx: &mut dyn PieceContext) {
        let [width, height, depth] = self.spec.dims();
        let (w, d) = (width - 1, depth - 1);
        self.patch_shell(ctx, [width, height, depth]);
        // Plank floor over the shaft's 3×3 interior; upstream's `replaceAirOrChains` floor.
        self.fill(ctx, [1, 0, 1], [w - 1, 0, d - 1], self.blocks.main);
        // Carve the climbable 3×3 core.
        self.carve(ctx, [1, 1, 1], [w - 1, height - 2, d - 1]);
        // Ladder on the back wall (`VerticalEntrance`: local 24 of 22..26), facing into the shaft.
        for y in 1..=(height - 5).max(1) {
            self.set(ctx, [2, y, 1], self.blocks.ladder(self.spec.facing));
        }
        // The doorway on the front wall: stone-brick frame, slab lintel, a two-high opening.
        self.fill(ctx, [1, 1, d], [w - 1, 2, d], self.blocks.stone_wall);
        self.set(ctx, [2, 3, d], self.blocks.stone_slab);
        self.carve(ctx, [2, 1, d], [2, 2, d]);
        // One leg per corner, so the shaft does not float.
        for (x, z) in [(0, 0), (0, d), (w, 0), (w, d)] {
            self.leg(ctx, x, z);
        }
    }

    fn write_corridor(&self, ctx: &mut dyn PieceContext) {
        let [width, height, depth] = runtime::CORRIDOR_DIMS;
        let (w, d) = (width - 1, depth - 1);
        self.patch_shell(ctx, [width, height, depth]);
        // The floor's own randomizer (`chanceReplaceNonAir(floorRandomizer)`).
        for x in 1..w {
            for z in 0..=d {
                let state = if self.roll(ctx) < self.palette.replacement_rate {
                    self.pick(ctx, &self.blocks.floor_random)
                } else {
                    self.blocks.main
                };
                self.set(ctx, [x, 0, z], state);
            }
        }
        // The vaulted carve: 7 wide to y 5, 5 wide at y 6, 3 wide at y 7.
        self.carve(ctx, [1, 1, 0], [w - 1, 5, d]);
        self.carve(ctx, [2, 6, 0], [w - 2, 6, d]);
        self.carve(ctx, [3, 7, 0], [w - 3, 7, d]);
        // Any air left in the floor is planked.
        for x in 1..w {
            for z in 0..=d {
                self.set_if_air(ctx, [x, 0, z], self.blocks.main);
            }
        }
        self.open_walls(ctx, w, d);
        self.corridor_supports(ctx, w, d);
        self.rails(ctx, w / 2, d);
        self.lanterns(ctx, w, d);
        self.gravel(ctx, w, d);
        self.vines(ctx, w, d);
    }

    /// Carve the wall where a branch or a room hangs off this segment.
    fn open_walls(&self, ctx: &mut dyn PieceContext, w: i32, d: i32) {
        let right = right_of(self.spec.facing);
        for opening in &self.spec.openings {
            let step = opening.side.step();
            let on_right_axis = step[0] == right.step()[0] && step[2] == right.step()[2];
            let x = if on_right_axis { w } else { 0 };
            for along in opening.along..opening.along + opening.len {
                if along < 0 || along > d {
                    continue;
                }
                self.carve(ctx, [x, 1, along], [x, 2, along]);
                self.set_if_air(ctx, [x, 0, along], self.blocks.main);
            }
        }
    }

    fn corridor_supports(&self, ctx: &mut dyn PieceContext, w: i32, d: i32) {
        let seed = runtime::piece_seed(&self.spec.bounds);
        for (z, big) in runtime::corridor_supports(d + 1, seed) {
            if z + 2 > d {
                continue;
            }
            // Upstream only builds a support where the ceiling above is covered.
            let mut covered = 0;
            for x in 2..=w - 2 {
                if !self.is_air_mut(ctx, [x, 7, z]) {
                    covered += 1;
                }
            }
            if covered < 2 {
                continue;
            }
            if big {
                for x in 1..=2 {
                    for dz in 0..=2 {
                        if self.roll(ctx) < 0.6 {
                            self.set(ctx, [x, 1, z + dz], self.blocks.slab);
                            self.set(ctx, [w - x, 1, z + dz], self.blocks.slab);
                        }
                    }
                }
                for x in [1, w - 1] {
                    self.set(ctx, [x, 1, z + 1], self.blocks.main);
                    self.set(ctx, [x, 4, z + 1], self.blocks.main);
                    self.fill(ctx, [x, 2, z + 1], [x, 3, z + 1], self.blocks.support);
                }
                self.fill(ctx, [2, 5, z + 1], [w - 2, 5, z + 1], self.blocks.main);
                for x in 2..=w - 2 {
                    if self.roll(ctx) < 0.4 {
                        self.set(ctx, [x, 5, z + 1], self.blocks.support);
                    }
                }
                self.cobwebs(ctx, [1, 1, z], [w - 1, 4, z + 2]);
            } else {
                for x in [2, w - 2] {
                    self.set(ctx, [x, 1, z], self.blocks.main);
                    self.set(ctx, [x, 2, z], self.blocks.support);
                    self.set(ctx, [x, 3, z], self.blocks.main);
                }
                self.fill(ctx, [3, 4, z], [w - 3, 4, z], self.blocks.main);
                for x in 3..=w - 3 {
                    if self.roll(ctx) < 0.5 {
                        self.set(ctx, [x, 4, z], self.blocks.support);
                    }
                }
                for x in 2..=w - 2 {
                    if self.roll(ctx) < 0.4 {
                        self.set(ctx, [x, 3, z], self.blocks.support);
                    }
                }
                self.set(ctx, [3, 3, z], self.blocks.support);
                self.set(ctx, [w - 3, 3, z], self.blocks.support);
                self.cobwebs(ctx, [2, 3, z - 1], [w - 2, 4, z + 1]);
            }
        }
    }

    fn cobwebs(&self, ctx: &mut dyn PieceContext, from: [i32; 3], to: [i32; 3]) {
        for y in from[1]..=to[1] {
            for x in from[0]..=to[0] {
                for z in from[2]..=to[2] {
                    if self.roll(ctx) < runtime::COBWEB_SPAWN_RATE {
                        self.set_if_air(ctx, [x, y, z], self.blocks.cobweb);
                    }
                }
            }
        }
    }

    /// `generateRails`: half the floor's centre line gets rail, then every twentieth (or every
    /// 26th) block along it turns a rail into a powered one. Two passes, as upstream does.
    fn rails(&self, ctx: &mut dyn PieceContext, x: i32, d: i32) {
        for z in 0..=d {
            let air = self.is_air_mut(ctx, [x, 1, z]);
            let floor_solid = self
                .get(ctx, [x, 0, z])
                .is_some_and(|state| !ctx.is_air(state));
            if air && floor_solid && self.roll(ctx) < 0.5 {
                self.set(ctx, [x, 1, z], self.blocks.rail);
            }
        }
        let mut since_powered = 0;
        for z in 0..=d {
            since_powered += 1;
            let rail_here = self
                .get(ctx, [x, 1, z])
                .is_some_and(|state| state == self.blocks.rail);
            if rail_here && (ctx.random().next_int_bound(20) == 0 || since_powered > 25) {
                self.set(ctx, [x, 1, z], self.blocks.powered_rail);
                since_powered = 0;
            }
        }
    }

    fn lanterns(&self, ctx: &mut dyn PieceContext, w: i32, d: i32) {
        let mut z = 0;
        while z <= d {
            let mut placed = false;
            for x in 3..=w - 3 {
                if self.roll(ctx) < runtime::LANTERN_SPAWN_RATE {
                    let ceiling = self
                        .get(ctx, [x, 8, z])
                        .is_some_and(|state| !ctx.is_air(state));
                    if ceiling {
                        self.set_if_air(ctx, [x, 7, z], self.blocks.lantern);
                        placed = true;
                        break;
                    }
                }
            }
            // Upstream jumps twenty blocks after a lantern so they stay sparse.
            z += if placed { 21 } else { 1 };
        }
    }

    fn gravel(&self, ctx: &mut dyn PieceContext, w: i32, d: i32) {
        let seed = runtime::piece_seed(&self.spec.bounds);
        for (z, right) in runtime::gravel_piles(d + 1, self.palette.decorations.gravel_pile, seed) {
            if z + 2 > d {
                continue;
            }
            let wall = if right { w - 1 } else { 1 };
            let inner = if right { w - 2 } else { 2 };
            for dz in 0..=2 {
                for y in 1..=2 {
                    self.set_if_air(ctx, [wall, y, z + dz], self.blocks.gravel);
                }
            }
            self.set_if_air(ctx, [wall, 3, z + 1], self.blocks.gravel);
            self.set_if_air(ctx, [inner, 1, z + 1], self.blocks.gravel);
        }
    }

    /// `addVines` over the four walls: a vine in an air cell whose neighbour is solid.
    fn vines(&self, ctx: &mut dyn PieceContext, w: i32, d: i32) {
        let chance = self.palette.decorations.vine;
        if chance <= 0.0 {
            return;
        }
        let right = right_of(self.spec.facing);
        for y in 1..=5 {
            for z in 0..=d {
                for (x, against) in [(1, right.opposite()), (w - 1, right)] {
                    if self.roll(ctx) >= chance {
                        continue;
                    }
                    let neighbour = self
                        .get(ctx, [if x == 1 { 0 } else { w }, y, z])
                        .is_some_and(|state| !ctx.is_air(state));
                    if neighbour && self.is_air_mut(ctx, [x, y, z]) {
                        self.set(ctx, [x, y, z], self.blocks.vine(against));
                    }
                }
            }
        }
    }

    fn write_branch(&self, ctx: &mut dyn PieceContext) {
        let [width, height, depth] = runtime::BRANCH_DIMS;
        let (w, d) = (width - 1, depth - 1);
        self.patch_shell(ctx, [width, height, depth]);
        for x in 1..w {
            for z in 0..=d {
                let state = if self.roll(ctx) < self.palette.replacement_rate {
                    self.pick(ctx, &self.blocks.floor_random)
                } else {
                    self.blocks.main
                };
                self.set(ctx, [x, 0, z], state);
            }
        }
        self.carve(ctx, [1, 1, 0], [w - 1, height - 2, d]);
        // The doorway into the corridor it hangs off.
        self.carve(ctx, [1, 1, 0], [w - 1, 2, 0]);
        for x in 1..w {
            for z in 0..=d {
                self.set_if_air(ctx, [x, 0, z], self.blocks.main);
            }
        }
        let seed = runtime::piece_seed(&self.spec.bounds);
        for z in runtime::branch_supports(d + 1, seed) {
            let mut covered = 0;
            for x in 1..=w - 1 {
                if !self.is_air_mut(ctx, [x, height - 1, z]) {
                    covered += 1;
                }
            }
            if covered < 2 {
                continue;
            }
            for x in [1, w - 1] {
                self.fill(ctx, [x, 1, z], [x, 2, z], self.blocks.support);
                self.set(ctx, [x, 3, z], self.blocks.main);
            }
            for x in 1..=w - 1 {
                if self.roll(ctx) < 0.25 {
                    self.set(ctx, [x, 3, z], self.blocks.support);
                }
            }
            self.cobwebs(ctx, [1, 3, z - 1], [w - 1, 3, z + 1]);
        }
        self.rails(ctx, w / 2, d);
        self.vines(ctx, w, d);
    }

    /// `SmallTunnelStairs.postProcess`: five descending slices, each with its own floor and a rail.
    fn write_stairs(&self, ctx: &mut dyn PieceContext) {
        let [width, height, depth] = runtime::STAIRS_DIMS;
        let (w, d) = (width - 1, depth - 1);
        // Randomize what the stair cuts through above the floor (`chanceReplaceNonAir` over the
        // box), then the floor itself with the floor randomizer.
        self.randomize_non_air(ctx, [width, height, depth], 1);
        for x in 0..=w {
            for z in 0..=d {
                let state = if self.roll(ctx) < self.palette.replacement_rate {
                    self.pick(ctx, &self.blocks.floor_random)
                } else {
                    self.blocks.main
                };
                self.set(ctx, [x, 0, z], state);
            }
        }
        for step in 0..depth {
            // The step's slice: three wide, four high, one block further along the run.
            self.carve(ctx, [1, step + 1, step], [w - 1, step + 4, step]);
            // Its floor: a half chance of the floor randomizer, then any air left is the main
            // block (`chanceReplaceNonAir(.5f, …)` then `replaceAirOrChains`).
            for x in 1..=w - 1 {
                let state = if self.roll(ctx) < 0.5 {
                    self.pick(ctx, &self.blocks.floor_random)
                } else {
                    self.blocks.main
                };
                self.set(ctx, [x, step, step], state);
            }
            for x in 1..=w - 1 {
                self.set_if_air(ctx, [x, step, step], self.blocks.main);
            }
            // The rail on the step's centre line: powered on the first, flat on the last.
            if self.roll(ctx) < 0.5 {
                let state = if step == 0 {
                    self.blocks.powered_stair_rail(self.spec.facing)
                } else {
                    self.blocks.stair_rail(self.spec.facing, step == depth - 1)
                };
                self.set(ctx, [2, step + 1, step], state);
            }
        }
        self.stairs_vines(ctx, w, d);
    }

    /// `addVines` over the stair's own bounds (upstream passes the piece's full height).
    fn stairs_vines(&self, ctx: &mut dyn PieceContext, w: i32, d: i32) {
        let chance = self.palette.decorations.vine;
        if chance <= 0.0 {
            return;
        }
        let right = right_of(self.spec.facing);
        for y in 1..=runtime::STAIRS_DIMS[1] - 1 {
            for z in 1..=d - 1 {
                for (x, against) in [(1, right.opposite()), (w - 1, right)] {
                    if self.roll(ctx) >= chance {
                        continue;
                    }
                    let neighbour = self
                        .get(ctx, [if x == 1 { 0 } else { w }, y, z])
                        .is_some_and(|state| !ctx.is_air(state));
                    if neighbour && self.is_air_mut(ctx, [x, y, z]) {
                        self.set(ctx, [x, y, z], self.blocks.vine(against));
                    }
                }
            }
        }
    }

    fn write_room(&self, ctx: &mut dyn PieceContext) {
        let [width, height, depth] = runtime::ROOM_DIMS;
        let (w, d) = (width - 1, depth - 1);
        // Brick floor over the whole footprint, then a brick wall round the room.
        for x in 0..width {
            for z in 0..depth {
                let state = self.pick(ctx, &self.blocks.brick_random);
                self.set(ctx, [x, 0, z], state);
            }
        }
        for y in 1..height {
            for x in 0..width {
                for z in 0..depth {
                    let edge = x == 0 || x == w || z == 0 || z == d;
                    if !edge {
                        continue;
                    }
                    let state = self.pick(ctx, &self.blocks.brick_random);
                    self.set(ctx, [x, y, z], state);
                }
            }
        }
        // Carve the room out, and its doorway on the side the corridor is.
        self.carve(ctx, [1, 1, 1], [w - 1, height - 2, d - 1]);
        self.carve(ctx, [4, 1, 0], [5, 2, 0]);
        // Patch the ceiling only where the terrain still has a block (upstream's `ceiling` pass).
        for x in 1..w {
            for z in 1..d {
                if !self.is_air_mut(ctx, [x, height - 1, z]) {
                    let state = self.pick(ctx, &self.blocks.brick_random);
                    self.set(ctx, [x, height - 1, z], state);
                }
            }
        }
        // Iron-bar columns under a covered ceiling, never adjacent (`generateIronBarSupports`).
        let mut previous_x = -2;
        for z in 2..=3 {
            for x in 2..=w - 2 {
                if x <= previous_x + 1 {
                    continue;
                }
                if self.is_air_mut(ctx, [x, height - 1, z]) {
                    continue;
                }
                if ctx.random().next_int_bound(5) != 0 {
                    continue;
                }
                self.fill(ctx, [x, 1, z], [x, 3, z], self.blocks.iron_bars);
                previous_x = x;
            }
        }
        // A crafting table, upstream's half-chance decoration.
        if self.roll(ctx) < 0.5 {
            self.set(ctx, [3, 1, 1], self.blocks.crafting_table);
        }
        // Legs under the corners, as upstream's `SideRoom.generateLegs` does whenever there is no
        // dungeon below (`hasDownstairs`) — and the dungeon is a recorded gap here.
        for (x, z) in [(1, 1), (1, d - 1), (w - 1, 1), (w - 1, d - 1)] {
            self.leg(ctx, x, z);
        }
    }
}

/// `StructurePiece.chance*` and the fixed-block writers the two new pieces share, in the piece's
/// own frame (local z runs into the piece's facing, x along its secondary axis).
impl Piece {
    fn state_of(
        &self,
        ctx: &mut dyn PieceContext,
        name: &str,
        properties: &[(&str, &str)],
    ) -> BlockState {
        match crate::support::key_for(name) {
            Some(key) => ctx.state_with(key, properties),
            None => self.blocks.air,
        }
    }

    /// The world direction a direction of upstream's north-facing frame points in for this piece:
    /// local +z is the piece's facing, local +x its secondary axis.
    fn world_dir(&self, java: &str) -> &'static str {
        let toward = |from: [i32; 3], to: [i32; 3]| {
            let step = [to[0] - from[0], to[2] - from[2]];
            match step {
                [1, 0] => "east",
                [-1, 0] => "west",
                [0, 1] => "south",
                _ => "north",
            }
        };
        let origin = self.spec.local_to_world([0, 0, 0]);
        let plus_z = toward(origin, self.spec.local_to_world([0, 0, 1]));
        let plus_x = toward(origin, self.spec.local_to_world([1, 0, 0]));
        let opposite = |name: &'static str| match name {
            "east" => "west",
            "west" => "east",
            "south" => "north",
            _ => "south",
        };
        match java {
            "north" => plus_z,
            "south" => opposite(plus_z),
            "east" => plus_x,
            _ => opposite(plus_x),
        }
    }

    /// Whether the piece's x axis runs against upstream's handedness, which swaps a stair's inner
    /// left and right.
    fn mirrored(&self) -> bool {
        let origin = self.spec.local_to_world([0, 0, 0]);
        let z = self.spec.local_to_world([0, 0, 1]);
        let x = self.spec.local_to_world([1, 0, 0]);
        let (dz, dx) = (
            [z[0] - origin[0], z[2] - origin[2]],
            [x[0] - origin[0], x[2] - origin[2]],
        );
        // Upstream's north frame has +z north (0, -1) and +x east (1, 0): a cross product of -1.
        dx[0] * dz[1] - dx[1] * dz[0] > 0
    }

    fn chance_fill(
        &self,
        ctx: &mut dyn PieceContext,
        chance: f32,
        from: [i32; 3],
        to: [i32; 3],
        state: BlockState,
    ) {
        for y in from[1]..=to[1] {
            for x in from[0]..=to[0] {
                for z in from[2]..=to[2] {
                    let local = [x, y, z];
                    if !self.visible(ctx, local) {
                        continue;
                    }
                    if self.roll(ctx) < chance {
                        self.set(ctx, local, state);
                    }
                }
            }
        }
    }

    /// `chanceReplaceNonAir`: a non-air block is replaced with `chance`.
    fn chance_replace_non_air(
        &self,
        ctx: &mut dyn PieceContext,
        chance: f32,
        from: [i32; 3],
        to: [i32; 3],
        state: BlockState,
    ) {
        for y in from[1]..=to[1] {
            for x in from[0]..=to[0] {
                for z in from[2]..=to[2] {
                    let local = [x, y, z];
                    if !self.visible(ctx, local) {
                        continue;
                    }
                    if let Some(current) = self.get(ctx, local)
                        && !ctx.is_air(current)
                        && self.roll(ctx) < chance
                    {
                        self.set(ctx, local, state);
                    }
                }
            }
        }
    }

    /// `chanceReplaceSolid`: a solid block is replaced with `chance`.
    fn chance_replace_solid(
        &self,
        ctx: &mut dyn PieceContext,
        chance: f32,
        from: [i32; 3],
        to: [i32; 3],
        state: BlockState,
    ) {
        for y in from[1]..=to[1] {
            for x in from[0]..=to[0] {
                for z in from[2]..=to[2] {
                    let local = [x, y, z];
                    if !self.visible(ctx, local) {
                        continue;
                    }
                    if self.roll(ctx) < chance
                        && let Some(current) = self.get(ctx, local)
                        && ctx.solid_render(current)
                    {
                        self.set(ctx, local, state);
                    }
                }
            }
        }
    }

    fn block_entity_at(&self, ctx: &mut dyn PieceContext, local: [i32; 3], nbt: Compound) {
        let pos = self.spec.local_to_world(local);
        if ctx.chunk_box().is_inside(pos) {
            ctx.block_entity(pos, nbt);
        }
    }

    /// `OreDeposit.postProcess`: a cobblestone shell round a pocket of one ore.
    fn write_ore_deposit(&self, ctx: &mut dyn PieceContext) {
        let [width, height, depth] = runtime::ORE_DEPOSIT_DIMS;
        let end = [width - 1, height - 1, depth - 1];
        let extra = &self.blocks.extra;
        let ore = extra.ores[self.spec.variant.clamp(0, 7) as usize];
        self.chance_replace_solid(ctx, 0.9, [0, 0, 0], end, extra.cobblestone);
        self.chance_replace_solid(
            ctx,
            0.65,
            [1, 1, 1],
            [end[0] - 1, end[1] - 1, end[2] - 1],
            ore,
        );
        self.chance_replace_solid(ctx, 0.15, [0, 0, 0], end, ore);
    }

    /// `ZombieVillagerRoom.postProcess`: a stone cell with an iron door, beds, an anvil and a
    /// zombie villager spawner under the floor.
    fn write_zombie_room(&self, ctx: &mut dyn PieceContext) {
        let [width, _, depth] = runtime::ZOMBIE_ROOM_DIMS;
        let (xe, ze) = (width - 1, depth - 1);
        let extra = &self.blocks.extra;
        let air = self.blocks.air;
        let walls = [
            ([1, 0, 0], [5, 2, 0]),
            ([0, 0, 1], [0, 2, 5]),
            ([xe, 0, 1], [xe, 2, 5]),
            ([1, 0, ze], [5, 2, ze]),
        ];
        for (from, to) in walls {
            self.fill(ctx, from, to, extra.stone);
        }
        for (from, to) in walls {
            self.chance_replace_non_air(ctx, 0.4, from, to, extra.cobblestone);
        }
        for (from, to) in walls {
            self.chance_replace_non_air(ctx, 0.1, from, to, extra.stone_bricks);
        }
        // Slabs on top of the outer walls.
        let tops = [
            ([2, 3, 0], [4, 3, 0]),
            ([0, 3, 2], [0, 3, 4]),
            ([xe, 3, 2], [xe, 3, 4]),
            ([2, 3, ze], [4, 3, ze]),
        ];
        for (from, to) in tops {
            self.fill(ctx, from, to, extra.stone_slab);
        }
        for (from, to) in tops {
            self.chance_fill(ctx, 0.5, from, to, extra.cobblestone_slab);
        }
        // The second layer, upside-down stairs round the ceiling, with inner corners.
        let stair = |ctx: &mut dyn PieceContext, facing: &str, shape: Option<&str>| {
            let facing = self.world_dir(facing);
            let mut properties = vec![("half", "top"), ("facing", facing)];
            if let Some(shape) = shape {
                properties.push(("shape", shape));
            }
            self.state_of(ctx, "minecraft:cobblestone_stairs", &properties)
        };
        let sides = [
            ([2, 3, 1], [4, 3, 1], "south"),
            ([1, 3, 2], [1, 3, 4], "west"),
            ([2, 3, 5], [4, 3, 5], "north"),
            ([5, 3, 2], [5, 3, 4], "east"),
        ];
        for (from, to, facing) in sides {
            let state = stair(ctx, facing, None);
            self.fill(ctx, from, to, state);
        }
        let (left, right) = if self.mirrored() {
            ("inner_right", "inner_left")
        } else {
            ("inner_left", "inner_right")
        };
        for (at, facing, shape) in [
            ([1, 3, 1], "south", right),
            ([1, 3, 5], "north", left),
            ([5, 3, 1], "south", left),
            ([5, 3, 5], "north", right),
        ] {
            let state = stair(ctx, facing, Some(shape));
            self.set(ctx, at, state);
        }
        // The third layer: bottom slabs, half of them cobblestone, and a top slab in the middle.
        self.fill(ctx, [2, 4, 2], [4, 4, 4], extra.stone_slab);
        self.chance_fill(ctx, 0.5, [2, 4, 2], [4, 4, 4], extra.cobblestone_slab);
        self.set(ctx, [3, 4, 3], air);
        self.set(ctx, [3, 4, 3], extra.top_stone_slab);
        // The floor.
        self.fill(ctx, [1, 0, 1], [5, 0, 5], extra.stone);
        self.chance_fill(ctx, 0.4, [1, 0, 1], [5, 0, 5], extra.cobblestone);
        self.chance_fill(ctx, 0.1, [1, 0, 1], [5, 0, 5], extra.stone_bricks);
        // The inside, and the door.
        self.fill(ctx, [1, 1, 1], [5, 2, 5], air);
        self.fill(ctx, [2, 3, 2], [4, 3, 4], air);
        self.fill(ctx, [3, 1, 0], [3, 2, 0], air);
        let facing = self.world_dir("north");
        let lower = self.state_of(
            ctx,
            "minecraft:iron_door",
            &[("facing", facing), ("half", "lower")],
        );
        let upper = self.state_of(
            ctx,
            "minecraft:iron_door",
            &[("facing", facing), ("half", "upper")],
        );
        self.set(ctx, [3, 1, 0], lower);
        self.set(ctx, [3, 2, 0], upper);
        // The window, the beds, the corner torch and the button.
        self.fill(ctx, [6, 2, 2], [6, 2, 4], extra.iron_bars);
        let foot = self.state_of(
            ctx,
            "minecraft:black_bed",
            &[("facing", facing), ("part", "foot")],
        );
        let head = self.state_of(
            ctx,
            "minecraft:black_bed",
            &[("facing", facing), ("part", "head")],
        );
        for x in [1, 5] {
            self.set(ctx, [x, 1, 4], foot);
            self.set(ctx, [x, 1, 5], head);
        }
        self.set(ctx, [1, 1, 1], extra.cobblestone_wall);
        self.set(ctx, [1, 2, 1], extra.redstone_torch);
        let button = self.state_of(
            ctx,
            "minecraft:stone_button",
            &[("face", "wall"), ("facing", self.world_dir("south"))],
        );
        self.set(ctx, [2, 2, 1], button);
        self.set(ctx, [5, 1, 2], extra.anvil);
        // A barrel of mineshaft loot, and the bench: smithing table, crafting table or blast furnace.
        self.set(ctx, [5, 1, 1], extra.barrel);
        let mut barrel = Compound::new();
        barrel.insert("id".into(), Tag::String("minecraft:barrel".into()));
        barrel.insert(
            "LootTable".into(),
            Tag::String("minecraft:chests/abandoned_mineshaft".into()),
        );
        self.block_entity_at(ctx, [5, 1, 1], barrel);
        let bench = if self.roll(ctx) < 0.33 {
            extra.smithing_table
        } else if self.roll(ctx) < 0.67 {
            self.blocks.crafting_table
        } else {
            extra.blast_furnace
        };
        self.set(ctx, [2, 1, 5], bench);
        self.chance_fill(ctx, 0.3, [2, 3, 2], [4, 3, 4], self.blocks.cobweb);
        // The spawner under the floor's middle.
        self.set(ctx, [3, 0, 3], extra.spawner);
        let mut spawner = Compound::new();
        spawner.insert("id".into(), Tag::String("minecraft:mob_spawner".into()));
        let mut entity = Compound::new();
        entity.insert("id".into(), Tag::String("minecraft:zombie_villager".into()));
        let mut data = Compound::new();
        data.insert("entity".into(), Tag::Compound(entity));
        spawner.insert("SpawnData".into(), Tag::Compound(data));
        self.block_entity_at(ctx, [3, 0, 3], spawner);
    }
}

fn right_of(facing: Facing) -> Facing {
    match facing {
        Facing::South => Facing::West,
        Facing::West => Facing::North,
        Facing::North => Facing::East,
        Facing::East => Facing::South,
    }
}

impl StructurePiece for Piece {
    fn bounding_box(&self) -> BoundingBox {
        BoundingBox::new(self.spec.bounds.min, self.spec.bounds.max)
    }

    fn post_process(&self, ctx: &mut dyn PieceContext) -> Result<(), StructureError> {
        match self.spec.kind {
            PieceKind::Entrance => self.write_entrance(ctx),
            PieceKind::Corridor => self.write_corridor(ctx),
            PieceKind::Branch => self.write_branch(ctx),
            PieceKind::Stairs => self.write_stairs(ctx),
            PieceKind::Room => self.write_room(ctx),
            PieceKind::OreDeposit => self.write_ore_deposit(ctx),
            PieceKind::ZombieRoom => self.write_zombie_room(ctx),
        }
        Ok(())
    }

    fn add_additional_save_data(&self, tag: &mut Compound) {
        tag.insert("kind".into(), Tag::String(self.spec.kind.name().into()));
        tag.insert("facing".into(), Tag::String(self.spec.facing.name().into()));
        tag.insert("depth".into(), Tag::Int(self.spec.depth));
        if self.spec.variant != 0 {
            tag.insert("variant".into(), Tag::Int(self.spec.variant));
        }
        let openings: Vec<Tag> = self
            .spec
            .openings
            .iter()
            .map(|opening| {
                Tag::IntArray(vec![opening.along, opening.len, facing_code(opening.side)])
            })
            .collect();
        if !openings.is_empty() {
            tag.insert("openings".into(), Tag::List(openings));
        }
        tag.insert("palette".into(), encode_palette(&self.palette));
    }
}

fn facing_code(facing: Facing) -> i32 {
    match facing {
        Facing::South => 0,
        Facing::West => 1,
        Facing::North => 2,
        Facing::East => 3,
    }
}

fn facing_from_code(code: i32) -> Option<Facing> {
    match code {
        0 => Some(Facing::South),
        1 => Some(Facing::West),
        2 => Some(Facing::North),
        3 => Some(Facing::East),
        _ => None,
    }
}

/// Read a piece's own fields from its saved tag. The engine's `BB` gives the bounds.
pub fn piece_from_tag(tag: &Compound) -> Result<(PieceSpec, Palette), StructureError> {
    // The engine hands the piece's whole compound; `Tag::get` is the key lookup with UTF-16 keys.
    let root = Tag::Compound(tag.clone());
    let kind = match root.get("kind") {
        Some(Tag::String(name)) => name
            .to_string()
            .ok()
            .and_then(|name| PieceKind::from_name(&name)),
        _ => None,
    }
    .ok_or_else(|| StructureError("bettermineshafts: a saved piece has no kind".into()))?;
    let facing = match root.get("facing") {
        Some(Tag::String(name)) => name
            .to_string()
            .ok()
            .and_then(|name| Facing::from_name(&name)),
        _ => None,
    }
    .ok_or_else(|| StructureError("bettermineshafts: a saved piece has no facing".into()))?;
    // A saved depth only selects generation behaviour; clamp it like the box.
    let depth = match root.get("depth") {
        Some(Tag::Int(depth)) => (*depth).clamp(0, runtime::MAX_SAVED_SPAN),
        _ => 0,
    };
    let variant = match root.get("variant") {
        Some(Tag::Int(variant)) => (*variant).clamp(0, runtime::ORE_WEIGHTS.len() as i32 - 1),
        _ => 0,
    };
    let box_ = match root.get("BB") {
        Some(Tag::IntArray(values)) if values.len() == 6 => Box3::new(
            [values[0], values[1], values[2]],
            [values[3], values[4], values[5]],
        ),
        _ => {
            return Err(StructureError(
                "bettermineshafts: a saved piece has no BB".into(),
            ));
        }
    };
    // Reject a corrupt or crafted box before any generator loop uses it: an
    // inverted, overflowing or attacker-sized span would make `patch_shell`
    // iterate a range no world can hold (the review finding).
    if !box_.is_plausible() {
        return Err(StructureError(
            "bettermineshafts: a saved piece box is outside the plausible span".into(),
        ));
    }
    let spans = box_
        .span_checked()
        .ok_or_else(|| StructureError("bettermineshafts: a saved piece box has no span".into()))?;
    let mut openings = Vec::new();
    if let Some(Tag::List(list)) = root.get("openings") {
        for entry in list {
            let Tag::IntArray(values) = entry else {
                return Err(StructureError(
                    "bettermineshafts: a saved opening is not an int array".into(),
                ));
            };
            if values.len() != 3 {
                return Err(StructureError(
                    "bettermineshafts: a saved opening is not [along, len, side]".into(),
                ));
            }
            let side = facing_from_code(values[2]).ok_or_else(|| {
                StructureError(format!(
                    "bettermineshafts: a saved opening names side {}",
                    values[2]
                ))
            })?;
            // Openings run along the piece's length (local z); clamp both ends
            // so `along..along + len` cannot overflow or run past the piece.
            let depth = spans[2];
            let along = values[0].clamp(0, depth.saturating_sub(1));
            let len = values[1].clamp(0, depth - along);
            if len == 0 {
                continue;
            }
            openings.push(Opening { along, len, side });
        }
    }
    let palette = root
        .get("palette")
        .ok_or_else(|| StructureError("bettermineshafts: a saved piece has no palette".into()))
        .and_then(decode_palette)?;
    Ok((
        PieceSpec {
            kind,
            bounds: box_,
            facing,
            depth,
            openings,
            variant,
        },
        palette,
    ))
}

fn encode_palette(palette: &Palette) -> Tag {
    let blocks = Tag::compound([
        ("main", encode_block(&palette.blocks.main)),
        ("support", encode_block(&palette.blocks.support)),
        ("slab", encode_block(&palette.blocks.slab)),
        ("gravel", encode_block(&palette.blocks.gravel)),
        ("stoneWall", encode_block(&palette.blocks.stone_wall)),
        ("stoneSlab", encode_block(&palette.blocks.stone_slab)),
        ("trapdoor", encode_block(&palette.blocks.trapdoor)),
        ("smallLeg", encode_block(&palette.blocks.small_leg)),
    ]);
    let decorations = palette.decorations;
    Tag::compound([
        (
            "replacementRate",
            Tag::Float(palette.replacement_rate.to_bits()),
        ),
        ("legVariant", Tag::String(palette.leg_variant.name().into())),
        (
            "decorations",
            Tag::compound([
                ("vine", Tag::Float(decorations.vine.to_bits())),
                ("snow", Tag::Float(decorations.snow.to_bits())),
                ("cactus", Tag::Float(decorations.cactus.to_bits())),
                ("deadBush", Tag::Float(decorations.dead_bush.to_bits())),
                ("mushroom", Tag::Float(decorations.mushroom.to_bits())),
                ("gravelPile", Tag::Float(decorations.gravel_pile.to_bits())),
                ("lush", Tag::Byte(i8::from(decorations.lush))),
                ("dripstone", Tag::Byte(i8::from(decorations.dripstone))),
            ]),
        ),
        ("blocks", blocks),
        (
            "randomizers",
            Tag::compound([
                ("main", encode_randomizer(&palette.randomizers.main)),
                ("floor", encode_randomizer(&palette.randomizers.floor)),
                ("brick", encode_randomizer(&palette.randomizers.brick)),
                ("leg", encode_randomizer(&palette.randomizers.leg)),
            ]),
        ),
    ])
}

fn encode_block(reference: &BlockRef) -> Tag {
    let properties: Vec<Tag> = reference
        .properties
        .iter()
        .map(|(name, value)| {
            Tag::compound([
                ("name", Tag::String(name.as_str().into())),
                ("value", Tag::String(value.as_str().into())),
            ])
        })
        .collect();
    Tag::compound([
        ("id", Tag::String(reference.name.as_str().into())),
        ("properties", Tag::List(properties)),
    ])
}

fn encode_randomizer(randomizer: &Randomizer) -> Tag {
    let entries: Vec<Tag> = randomizer
        .entries
        .iter()
        .map(|entry| {
            Tag::compound([
                ("state", encode_block(&entry.state)),
                ("probability", Tag::Float(entry.probability.to_bits())),
            ])
        })
        .collect();
    Tag::compound([
        ("default", encode_block(&randomizer.default)),
        ("entries", Tag::List(entries)),
    ])
}

/// Decode a saved palette strictly: a missing or malformed field is a typed error, never a default.
pub fn decode_palette(tag: &Tag) -> Result<Palette, StructureError> {
    let blocks = tag
        .get("blocks")
        .ok_or_else(|| StructureError("bettermineshafts: a palette has no blocks".into()))?;
    let randomizers = tag
        .get("randomizers")
        .ok_or_else(|| StructureError("bettermineshafts: a palette has no randomizers".into()))?;
    let decorations = tag
        .get("decorations")
        .ok_or_else(|| StructureError("bettermineshafts: a palette has no decorations".into()))?;
    let leg_variant = match tag.get("legVariant") {
        Some(Tag::String(name)) => name
            .to_string()
            .ok()
            .and_then(|name| LegVariant::from_name(&name))
            .ok_or_else(|| {
                StructureError("bettermineshafts: a saved legVariant is unknown".into())
            })?,
        _ => LegVariant::Edge,
    };
    Ok(Palette {
        replacement_rate: decode_f32(tag, "replacementRate").unwrap_or(0.6),
        leg_variant,
        decorations: runtime::DecorationChances {
            vine: decode_f32(decorations, "vine").unwrap_or(0.0),
            snow: decode_f32(decorations, "snow").unwrap_or(0.0),
            cactus: decode_f32(decorations, "cactus").unwrap_or(0.0),
            dead_bush: decode_f32(decorations, "deadBush").unwrap_or(0.0),
            mushroom: decode_f32(decorations, "mushroom").unwrap_or(0.0),
            gravel_pile: decode_f32(decorations, "gravelPile").unwrap_or(0.0),
            lush: decode_bool(decorations, "lush"),
            dripstone: decode_bool(decorations, "dripstone"),
        },
        blocks: runtime::BlockSet {
            main: decode_block(blocks, "main")?,
            support: decode_block(blocks, "support")?,
            slab: decode_block(blocks, "slab")?,
            gravel: decode_block(blocks, "gravel")?,
            stone_wall: decode_block(blocks, "stoneWall")?,
            stone_slab: decode_block(blocks, "stoneSlab")?,
            trapdoor: decode_block(blocks, "trapdoor")?,
            small_leg: decode_block(blocks, "smallLeg")?,
        },
        randomizers: runtime::RandomizerSet {
            main: decode_randomizer(randomizers, "main")?,
            floor: decode_randomizer(randomizers, "floor")?,
            brick: decode_randomizer(randomizers, "brick")?,
            leg: decode_randomizer(randomizers, "leg")?,
        },
    })
}

fn decode_block(parent: &Tag, key: &str) -> Result<BlockRef, StructureError> {
    let entry = parent
        .get(key)
        .ok_or_else(|| StructureError(format!("bettermineshafts: a palette has no block {key}")))?;
    let name = match entry.get("id") {
        Some(Tag::String(name)) => name
            .to_string()
            .map_err(|_| StructureError("bettermineshafts: a block name is not UTF-8".into()))?,
        _ => {
            return Err(StructureError(format!(
                "bettermineshafts: block {key} has no id"
            )));
        }
    };
    let mut properties = Vec::new();
    if let Some(Tag::List(list)) = entry.get("properties") {
        for property in list {
            let name = match property.get("name") {
                Some(Tag::String(value)) => value.to_string().map_err(|_| {
                    StructureError("bettermineshafts: a property name is not UTF-8".into())
                })?,
                _ => {
                    return Err(StructureError(
                        "bettermineshafts: a property has no name".into(),
                    ));
                }
            };
            let value = match property.get("value") {
                Some(Tag::String(value)) => value.to_string().map_err(|_| {
                    StructureError("bettermineshafts: a property value is not UTF-8".into())
                })?,
                _ => {
                    return Err(StructureError(
                        "bettermineshafts: a property has no value".into(),
                    ));
                }
            };
            properties.push((name, value));
        }
    }
    Ok(BlockRef { name, properties })
}

fn decode_randomizer(parent: &Tag, key: &str) -> Result<Randomizer, StructureError> {
    let entry = parent.get(key).ok_or_else(|| {
        StructureError(format!(
            "bettermineshafts: a palette has no randomizer {key}"
        ))
    })?;
    let default = decode_block(entry, "default")?;
    let mut entries = Vec::new();
    if let Some(Tag::List(list)) = entry.get("entries") {
        for weighted in list {
            let state = decode_block(weighted, "state")?;
            let probability = decode_f32(weighted, "probability").unwrap_or(0.0);
            entries.push(Weighted { state, probability });
        }
    }
    Ok(Randomizer { default, entries })
}

fn decode_f32(parent: &Tag, key: &str) -> Option<f32> {
    match parent.get(key) {
        Some(Tag::Float(bits)) => Some(f32::from_bits(*bits)),
        Some(Tag::Double(bits)) => Some(f64::from_bits(*bits) as f32),
        Some(Tag::Int(value)) => Some(*value as f32),
        Some(Tag::Long(value)) => Some(*value as f32),
        _ => None,
    }
}

fn decode_bool(parent: &Tag, key: &str) -> bool {
    matches!(parent.get(key), Some(Tag::Byte(value)) if *value != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{BlockSet, DecorationChances, RandomizerSet};

    fn sample_palette() -> Palette {
        let mut palette = Palette::oak_defaults();
        palette.replacement_rate = 0.4;
        palette.leg_variant = LegVariant::Inner;
        palette.decorations = DecorationChances {
            vine: 0.25,
            snow: 0.5,
            cactus: 0.75,
            dead_bush: 0.125,
            mushroom: 0.0625,
            gravel_pile: 0.2,
            lush: true,
            dripstone: false,
        };
        palette.blocks = BlockSet {
            main: BlockRef::with("minecraft:acacia_planks", &[("waterlogged", "false")]),
            support: BlockRef::new("minecraft:acacia_fence"),
            slab: BlockRef::new("minecraft:acacia_slab"),
            gravel: BlockRef::new("minecraft:gravel"),
            stone_wall: BlockRef::new("minecraft:cobblestone_wall"),
            stone_slab: BlockRef::new("minecraft:cobblestone_slab"),
            trapdoor: BlockRef::new("minecraft:acacia_trapdoor"),
            small_leg: BlockRef::with("minecraft:stripped_acacia_log", &[("axis", "y")]),
        };
        palette.randomizers = RandomizerSet {
            main: Randomizer {
                default: BlockRef::new("minecraft:acacia_planks"),
                entries: vec![
                    Weighted {
                        state: BlockRef::new("minecraft:cobblestone"),
                        probability: 0.15,
                    },
                    Weighted {
                        state: BlockRef::new("minecraft:sandstone"),
                        probability: 0.05,
                    },
                ],
            },
            floor: Randomizer {
                default: BlockRef::new("minecraft:sand"),
                entries: Vec::new(),
            },
            brick: Randomizer {
                default: BlockRef::new("minecraft:sandstone"),
                entries: Vec::new(),
            },
            leg: Randomizer {
                default: BlockRef::with("minecraft:stripped_acacia_log", &[("axis", "y")]),
                entries: Vec::new(),
            },
        };
        palette
    }

    #[test]
    fn a_palette_round_trips_through_its_saved_tag() {
        let palette = sample_palette();
        let tag = encode_palette(&palette);
        let decoded = decode_palette(&tag).expect("the encoded palette decodes");
        assert_eq!(decoded, palette);
        // Floats are saved as their bits, so the round trip is exact, not approximate.
        assert_eq!(decoded.replacement_rate, 0.4);
        assert_eq!(decoded.decorations.mushroom, 0.0625);
        assert!(decoded.decorations.lush && !decoded.decorations.dripstone);
        assert_eq!(decoded.blocks.main.property("waterlogged"), Some("false"));
    }

    #[test]
    fn a_saved_piece_box_outside_the_plausible_span_is_rejected() {
        let mut tag = Compound::new();
        tag.insert("kind".into(), Tag::String("corridor".into()));
        tag.insert("facing".into(), Tag::String("north".into()));
        tag.insert("BB".into(), Tag::IntArray(vec![0, 0, 0, 1_000_000, 0, 0]));
        tag.insert("palette".into(), encode_palette(&Palette::oak_defaults()));
        let error = piece_from_tag(&tag).expect_err("an attacker-sized box");
        assert!(error.to_string().contains("plausible"), "{error}");

        tag.insert("BB".into(), Tag::IntArray(vec![1, 0, 0, 0, 0, 0]));
        assert!(piece_from_tag(&tag).is_err(), "an inverted box");
    }

    #[test]
    fn a_saved_opening_is_clamped_to_the_piece() {
        let mut tag = Compound::new();
        tag.insert("kind".into(), Tag::String("corridor".into()));
        tag.insert("facing".into(), Tag::String("west".into()));
        tag.insert("BB".into(), Tag::IntArray(vec![-8, -20, -24, 0, -13, 0]));
        tag.insert(
            "openings".into(),
            Tag::List(vec![
                Tag::IntArray(vec![i32::MAX, i32::MAX, 0]),
                Tag::IntArray(vec![-5, 3, 0]),
                Tag::IntArray(vec![20, 0, 0]),
            ]),
        );
        tag.insert("palette".into(), encode_palette(&Palette::oak_defaults()));
        let (spec, _) = piece_from_tag(&tag).expect("the piece decodes");
        let depth = spec.bounds.span_checked().expect("a valid box")[2];
        assert!(!spec.openings.is_empty());
        for opening in &spec.openings {
            assert!(opening.along >= 0 && opening.along < depth, "{opening:?}");
            assert!(opening.len > 0 && opening.along + opening.len <= depth);
        }
        assert_eq!(spec.openings.len(), 2, "an empty length is dropped");
    }

    #[test]
    fn a_saved_piece_without_its_box_or_palette_is_a_typed_error_never_a_panic() {
        let empty = Compound::new();
        assert!(piece_from_tag(&empty).is_err());

        let mut no_box = Compound::new();
        no_box.insert("kind".into(), Tag::String("corridor".into()));
        no_box.insert("facing".into(), Tag::String("north".into()));
        no_box.insert("palette".into(), encode_palette(&Palette::oak_defaults()));
        let error = piece_from_tag(&no_box).expect_err("no BB");
        assert!(error.to_string().contains("BB"), "{error}");

        let mut unknown = Compound::new();
        unknown.insert("kind".into(), Tag::String("staircase".into()));
        let error = piece_from_tag(&unknown).expect_err("unknown kind");
        assert!(error.to_string().contains("kind"), "{error}");
    }

    #[test]
    fn a_saved_piece_reads_its_kind_facing_box_and_openings_back() {
        let mut tag = Compound::new();
        tag.insert("kind".into(), Tag::String("corridor".into()));
        tag.insert("facing".into(), Tag::String("west".into()));
        tag.insert("depth".into(), Tag::Int(2));
        tag.insert("BB".into(), Tag::IntArray(vec![-8, -20, -24, 0, -13, 0]));
        tag.insert(
            "openings".into(),
            Tag::List(vec![Tag::IntArray(vec![4, 3, 0])]),
        );
        tag.insert("palette".into(), encode_palette(&Palette::oak_defaults()));
        let (spec, palette) = piece_from_tag(&tag).expect("the piece decodes");
        assert_eq!(spec.kind, PieceKind::Corridor);
        assert_eq!(spec.facing, Facing::West);
        assert_eq!(spec.depth, 2);
        assert_eq!(spec.bounds, Box3::new([-8, -20, -24], [0, -13, 0]));
        assert_eq!(
            spec.openings,
            vec![Opening {
                along: 4,
                len: 3,
                side: Facing::South
            }]
        );
        assert_eq!(palette, Palette::oak_defaults());
    }

    #[test]
    fn every_block_the_carried_configs_name_is_requested() {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/bettermineshafts/worldgen/structure");
        let mut files = 0;
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&directory).expect("the carried structure directory") {
            let path = entry.expect("a directory entry").path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            files += 1;
            let text = std::fs::read_to_string(&path).expect("a carried structure file");
            for (index, _) in text.match_indices("\"id\": \"") {
                let rest = &text[index + 7..];
                let Some(end) = rest.find('"') else {
                    continue;
                };
                let name = &rest[..end];
                if !names.iter().any(|known| known == name) {
                    names.push(name.to_owned());
                }
            }
        }
        assert_eq!(files, 13, "the jar carries thirteen structure definitions");
        assert!(
            names.len() >= 60,
            "the configs name the variants' palettes: {}",
            names.len()
        );
        for name in names {
            assert!(
                crate::support::REQUESTED_BLOCKS.contains(&name.as_str()),
                "{name} is in a carried config but not requested at startup"
            );
        }
    }
}
