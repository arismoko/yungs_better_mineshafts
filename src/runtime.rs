//! The mineshaft's own numbers, palette and layout — pure Rust, no `mod_api` type, so
//! `cargo test` proves it without a game.
//!
//! Every constant here is read from the artefact, not invented:
//!
//! | Source | File | What it gives |
//! |---|---|---|
//! | jar / upstream `682a598` | `world/generator/pieces/{BigTunnel,SmallTunnel,SmallTunnelStairs,SideRoom,VerticalEntrance}.java` | piece boxes `9×8×24`, `5×5×8`, `5×9×5`, `10×5×5`, the 5×5 entrance shaft and its ladder/doorway |
//! | upstream `682a598` | `world/generator/BetterMineshaftGenerator.java` | chain rules (`chainLength > 3` stops the main corridor, roll `>= 10` or a young chain continues, `smallShaftPieceChainLength = 9`, `rand >= 80` places a stair) |
//! | upstream `682a598` | `module/ConfigModule.java` | `minY -55`, `maxY 30`, the spawn rates below |
//! | jar | `data/bettermineshafts/worldgen/structure/mineshaft_oak.json` | the oak palette, `replacementRate 0.6`, `legVariant edge`, `vineChance 0.1`, `gravelPileChance 0.1` |
//!
//! The notes record what is *not* here (turns, intersections, ore deposits, zombie-villager rooms,
//! the terrain-driven surface tunnel, minecarts, biome decorations).
//!
//! ## Upstream defect fixed here
//!
//! `BetterMineshaftStructure.findGenerationPoint` draws the start's height **twice**: once for the
//! `GenerationStub`'s position (`:42`) and again inside `generatePieces` for the piece
//! (`:62`), from the same `context.random()`. The stub's position and the entrance's floor are
//! therefore two different Y values for every mineshaft. [`assemble`] takes one floor and uses it
//! for both.

/// The mod id, which is also the resource namespace.
pub const MOD_ID: &str = "bettermineshafts";
/// The structure type registered in code (`register_structure_type("mineshaft", …)`).
pub const TYPE_ID: &str = "bettermineshafts:mineshaft";
/// The one structure this slice proves in the level: the oak variant, the jar's default.
pub const STRUCTURE_ID: &str = "bettermineshafts:mineshaft_oak";

/// `ConfigModule.minY` / `maxY`: the floor of a mineshaft is drawn from this range.
pub const CONFIG_MIN_Y: i32 = -55;
pub const CONFIG_MAX_Y: i32 = 30;

/// `ConfigModule.SpawnRates` (defaults; the mod's own config file is not data).
pub const LANTERN_SPAWN_RATE: f32 = 0.0067;
pub const TORCH_SPAWN_RATE: f32 = 0.02;
pub const WORKSTATION_SPAWN_RATE: f32 = 0.025;
pub const SMALL_SHAFT_SPAWN_RATE: f32 = 0.07;
pub const COBWEB_SPAWN_RATE: f32 = 0.15;
/// `ConfigModule.SpawnRates`' minecart rates, per block along a tunnel's middle: the main corridor
/// (`BigTunnel`) and a small shaft (`SmallTunnel`).
pub const MAIN_SHAFT_CHEST_MINECART_RATE: f32 = 0.01;
pub const MAIN_SHAFT_TNT_MINECART_RATE: f32 = 0.0025;
pub const SMALL_SHAFT_CHEST_MINECART_RATE: f32 = 0.00125;
pub const SMALL_SHAFT_TNT_MINECART_RATE: f32 = 0.0025;
pub const SMALL_SHAFT_PIECE_CHAIN_LENGTH: i32 = 9;

/// `BigTunnel.SECONDARY_AXIS_LEN`, `Y_AXIS_LEN`, `MAIN_AXIS_LEN` (secondary × height × length).
pub const CORRIDOR_DIMS: [i32; 3] = [9, 8, 24];
/// `SmallTunnel`'s bounds.
pub const BRANCH_DIMS: [i32; 3] = [5, 5, 8];
/// `SmallTunnelStairs`' bounds: 5 wide, 9 high (the passage it descends), 5 long.
pub const STAIRS_DIMS: [i32; 3] = [5, 9, 5];
/// `BetterMineshaftGenerator.generateAndAddSmallTunnelPiece`: a `nextInt(100)` at or above this
/// places a stair instead of a straight segment.
pub const STAIRS_ROLL_MIN: i32 = 80;
/// `SmallTunnelStairs.addChildren`: the run after a stair resumes four blocks higher (`minY + 4`).
pub const STAIRS_RISE: i32 = 4;
/// `SideRoom`'s bounds: 10 along the corridor, 5 deep, 5 high.
pub const ROOM_DIMS: [i32; 3] = [10, 5, 5];
/// `OreDeposit`'s box: 5 wide, 5 high, 4 long.
pub const ORE_DEPOSIT_DIMS: [i32; 3] = [5, 5, 4];
/// `ZombieVillagerRoom`'s box: 7 by 5 by 7.
pub const ZOMBIE_ROOM_DIMS: [i32; 3] = [7, 5, 7];
/// `ConfigModule.Ores`' defaults: cobble, coal, iron, redstone, gold, lapis, emerald, diamond.
pub const ORE_WEIGHTS: [i32; 8] = [50, 20, 9, 7, 7, 3, 3, 1];
/// `ConfigModule.SpawnRates.zombieVillagerRoomSpawnRate`: out of 100, at a branch's end.
pub const ZOMBIE_ROOM_SPAWN_RATE: i32 = 2;

/// The ore a `nextInt(100)` draw picks (`OreDeposit.addChildren`): the index into
/// [`ORE_WEIGHTS`] whose running total first passes the draw, cobble when none does.
pub fn ore_for_roll(roll: i32) -> i32 {
    let mut total = 0;
    for (index, weight) in ORE_WEIGHTS.iter().enumerate() {
        total += weight;
        if roll < total {
            return index as i32;
        }
    }
    0
}
/// `VerticalEntrance.SHAFT_LOCAL_XZ_START..=END` is 22..=26 in a 49-wide bounds: a 5×5 shaft.
pub const SHAFT_DIMS: [i32; 3] = [5, 5, 5];
/// `BetterMineshaftGenerator.generateAndAddBigTunnelPiece`: `chainLength > 3` stops the corridor.
pub const MAX_CORRIDOR_DEPTH: i32 = 3;
/// The most pieces one start may emit. A start is meant to be a mine, not a level: the main
/// corridor is bounded by [`MAX_CORRIDOR_DEPTH`] upstream, and the branches are measured against
/// this budget so a single `/place` cannot walk an unbounded piece tree.
pub const MAX_PIECES: usize = 32;
/// The straight run of a branch corridor, in segments (upstream allows 9 pieces *and* turns).
pub const MAX_BRANCH_SEGMENTS: usize = 3;
/// The required biome entries of `has_structure/better_mineshaft_oak.json` (the other three are
/// optional Terralith/Vibrant Journeys/NeoForge entries).
pub const BIOME_ENTRIES: [&str; 4] = [
    "minecraft:plains",
    "minecraft:sunflower_plains",
    "minecraft:swamp",
    "minecraft:meadow",
];
/// The optional entries beside them, kept in the carried tag and counted by the readout.
pub const OPTIONAL_BIOME_ENTRIES: usize = 3;

/// The readout's probe: a fixed seed, chunk, floor and surface, so `/bettermineshafts status`
/// answers the same piece count on every run. It is a probe, not a claim about a real column.
pub const PROBE_SEED: i64 = 1_274;
pub const PROBE_CHUNK: [i32; 2] = [0, 0];
pub const PROBE_FLOOR_Y: i32 = 0;
pub const PROBE_SURFACE_Y: i32 = 48;

/// A random source in the subset the layout draws from, so the layout is testable without a game.
pub trait Roll {
    /// `nextInt(bound)`, 0 for a bound below 1.
    fn next_i32(&mut self, bound: i32) -> i32;
    /// `nextFloat`.
    fn next_f32(&mut self) -> f32;
}

/// Vanilla's worldgen LCG (`LegacyRandomSource`/`WorldgenRandom`): the same seed and chunk give the
/// same assembly, which is what the readout and its tests rely on. In the game the layout draws
/// from the engine's `RandomSource` instead; this type exists for the probe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VanillaRandom {
    seed: i64,
}

impl VanillaRandom {
    pub fn new(seed: i64) -> Self {
        Self { seed }
    }

    /// `WorldgenRandom.setLargeFeatureSeed`.
    pub fn set_large_feature_seed(&mut self, world_seed: i64, chunk_x: i32, chunk_z: i32) {
        self.seed = world_seed
            ^ i64::from(chunk_x).wrapping_mul(3_418_731_287_142_i64)
            ^ i64::from(chunk_z).wrapping_mul(132_897_987_541_i64);
    }

    /// A whole horizontal facing from the same draw `Direction.Plane.HORIZONTAL` makes.
    pub fn facing(&mut self) -> Facing {
        Facing::from_roll(self.next_i32(4))
    }

    fn advance(&mut self) {
        self.seed = self
            .seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
    }

    fn next_bits(&mut self, bits: u32) -> i32 {
        self.advance();
        // Vanilla's `next(bits)` is `(int)(seed >>> (64 - bits))`: a LOGICAL shift, so the result is
        // never negative. A signed shift makes `nextInt(100)` negative (a corridor chain that stops
        // after two segments) and `nextFloat()` negative (a branch on most segments).
        ((self.seed as u64) >> (64 - bits)) as i32
    }
}

impl Roll for VanillaRandom {
    fn next_i32(&mut self, bound: i32) -> i32 {
        if bound <= 0 {
            return 0;
        }
        let mut value = self.next_bits(31);
        let mut remainder = value % bound;
        // Vanilla's overflow guard, in Java's wrapping arithmetic.
        if value.wrapping_sub(remainder).wrapping_add(bound - 1) < 0 {
            while value.wrapping_sub(remainder).wrapping_add(bound - 1) < 0 {
                value = self.next_bits(31);
                remainder = value % bound;
            }
        }
        remainder
    }

    fn next_f32(&mut self) -> f32 {
        self.next_bits(24) as f32 / (1_i32 << 24) as f32
    }
}

/// A horizontal direction, in vanilla's `Direction.from2DDataValue` order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    South,
    West,
    North,
    East,
}

impl Facing {
    pub fn from_roll(roll: i32) -> Self {
        match roll {
            1 => Self::West,
            2 => Self::North,
            3 => Self::East,
            _ => Self::South,
        }
    }

    /// One block along the facing.
    pub fn step(self) -> [i32; 3] {
        match self {
            Self::South => [0, 0, 1],
            Self::West => [-1, 0, 0],
            Self::North => [0, 0, -1],
            Self::East => [1, 0, 0],
        }
    }

    /// One block to the right of the facing, the secondary axis a piece's width runs along.
    pub fn right(self) -> [i32; 3] {
        match self {
            Self::South | Self::North => [1, 0, 0],
            Self::East | Self::West => [0, 0, 1],
        }
    }

    /// Which of the two horizontal axes (0 for x, 2 for z) this direction runs along.
    pub fn axis(self) -> usize {
        match self {
            Self::South | Self::North => 2,
            Self::East | Self::West => 0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::South => "south",
            Self::West => "west",
            Self::North => "north",
            Self::East => "east",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            "north" => Some(Self::North),
            "east" => Some(Self::East),
            _ => None,
        }
    }

    /// The opposite direction, the side of a wall a vine hangs from.
    pub fn opposite(self) -> Self {
        match self {
            Self::South => Self::North,
            Self::West => Self::East,
            Self::North => Self::South,
            Self::East => Self::West,
        }
    }
}

/// An inclusive integer bounds, the shape a piece's placement uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Box3 {
    pub min: [i32; 3],
    pub max: [i32; 3],
}

/// The largest span a saved piece box may have on one axis. A legitimate piece
/// is at most the corridor's 24; a corrupt or crafted world otherwise makes the
/// generator iterate an attacker-sized range.
pub const MAX_SAVED_SPAN: i32 = 64;

impl Box3 {
    pub fn new(min: [i32; 3], max: [i32; 3]) -> Self {
        Self { min, max }
    }

    pub fn span(&self) -> [i32; 3] {
        std::array::from_fn(|axis| self.max[axis] - self.min[axis] + 1)
    }

    /// The span computed without `i32` overflow, or `None` when an axis is
    /// inverted or does not fit.
    pub fn span_checked(&self) -> Option<[i32; 3]> {
        let mut spans = [0; 3];
        for (axis, span) in spans.iter_mut().enumerate() {
            let len = i64::from(self.max[axis]) - i64::from(self.min[axis]) + 1;
            if len < 1 {
                return None;
            }
            *span = i32::try_from(len).ok()?;
        }
        Some(spans)
    }

    /// Whether every axis is between 1 and [`MAX_SAVED_SPAN`] blocks long.
    pub fn is_plausible(&self) -> bool {
        self.span_checked()
            .is_some_and(|spans| spans.iter().all(|span| *span <= MAX_SAVED_SPAN))
    }

    pub fn is_inside(&self, position: [i32; 3]) -> bool {
        (0..3).all(|axis| position[axis] >= self.min[axis] && position[axis] <= self.max[axis])
    }

    pub fn intersects(&self, other: &Self) -> bool {
        (0..3).all(|axis| self.max[axis] >= other.min[axis] && self.min[axis] <= other.max[axis])
    }
}

/// Which piece a spec places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceKind {
    /// The vertical shaft, its ladder, doorway and legs.
    Entrance,
    /// The 9×8×24 main corridor.
    Corridor,
    /// The 5×5×8 branch corridor.
    Branch,
    /// The 5×9×5 stair that takes a branch four blocks lower.
    Stairs,
    /// The 10×5×5 workstation room.
    Room,
    /// The 5×5×4 pocket of ore at the end of a branch (`OreDeposit`).
    OreDeposit,
    /// The 7×5×7 room with an iron door and a zombie villager spawner (`ZombieVillagerRoom`).
    ZombieRoom,
}

impl PieceKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Entrance => "entrance",
            Self::Corridor => "corridor",
            Self::Branch => "branch",
            Self::Stairs => "stairs",
            Self::Room => "room",
            Self::OreDeposit => "ore_deposit",
            Self::ZombieRoom => "zombie_room",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "entrance" => Some(Self::Entrance),
            "corridor" => Some(Self::Corridor),
            "branch" => Some(Self::Branch),
            "stairs" => Some(Self::Stairs),
            "room" => Some(Self::Room),
            "ore_deposit" => Some(Self::OreDeposit),
            "zombie_room" => Some(Self::ZombieRoom),
            _ => None,
        }
    }
}

/// An opening a corridor carves in its own side wall for an attached piece: `len` blocks of the
/// wall, `along` blocks into the corridor, on the `side` the piece hangs off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opening {
    pub along: i32,
    pub len: i32,
    pub side: Facing,
}

/// One piece of an assembly: what it places, where, and which way its local frame runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PieceSpec {
    pub kind: PieceKind,
    pub bounds: Box3,
    pub facing: Facing,
    /// The chain depth (`genDepth` upstream) the piece was added at.
    pub depth: i32,
    /// Corridors only: the wall openings for the branches and rooms hanging off this segment.
    pub openings: Vec<Opening>,
    /// An ore deposit's ore, an index into [`ORE_WEIGHTS`]; 0 for every other kind.
    pub variant: i32,
}

impl PieceSpec {
    /// The world position of a local cell: secondary `side` along the right vector, `y` up from
    /// the floor, `along` from the piece's near face into its facing.
    pub fn local_to_world(&self, local: [i32; 3]) -> [i32; 3] {
        let secondary = self.facing.right();
        let secondary_axis = if secondary[0] != 0 { 0 } else { 2 };
        let main_axis = self.facing.axis();
        let along = match self.facing {
            Facing::South | Facing::East => self.bounds.min[main_axis] + local[2],
            Facing::North => self.bounds.max[main_axis] - local[2],
            Facing::West => self.bounds.max[main_axis] - local[2],
        };
        let mut world = [0; 3];
        world[secondary_axis] = self.bounds.min[secondary_axis] + local[0];
        world[1] = self.bounds.min[1] + local[1];
        world[main_axis] = along;
        world
    }

    /// The piece's bounds from its local frame's two extreme corners, for tests and readouts.
    pub fn corners(&self) -> (Box3, Box3) {
        let dims = self.dims();
        let first = self.local_to_world([0, 0, 0]);
        let last = self.local_to_world([dims[0] - 1, dims[1] - 1, dims[2] - 1]);
        (self.bounds, first_inclusive(first, last))
    }

    pub fn dims(&self) -> [i32; 3] {
        let [width, height, length] = match self.kind {
            PieceKind::Entrance => SHAFT_DIMS,
            PieceKind::Corridor => CORRIDOR_DIMS,
            PieceKind::Branch => BRANCH_DIMS,
            PieceKind::Stairs => STAIRS_DIMS,
            PieceKind::Room => ROOM_DIMS,
            PieceKind::OreDeposit => ORE_DEPOSIT_DIMS,
            PieceKind::ZombieRoom => ZOMBIE_ROOM_DIMS,
        };
        let height = if self.kind == PieceKind::Entrance {
            self.bounds.span()[1].max(height)
        } else {
            height
        };
        [width, height, length]
    }
}

fn first_inclusive(a: [i32; 3], b: [i32; 3]) -> Box3 {
    Box3::new(
        std::array::from_fn(|axis| a[axis].min(b[axis])),
        std::array::from_fn(|axis| a[axis].max(b[axis])),
    )
}

/// Why an assembly could not be built, or stopped early.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The build asked for fewer pieces than an entrance and one corridor need.
    Budget { needed: usize, budget: usize },
    /// The surface is too low over this column: a shaft shorter than four blocks cannot be climbed.
    Shaft { floor: i32, ceiling: i32 },
    /// The floor was clamped into the level's own height range.
    FloorClamped { asked: i32, clamped: i32 },
}

/// A start's pieces for one chunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assembly {
    pub start: [i32; 3],
    pub pieces: Vec<PieceSpec>,
    /// True when the budget stopped the chain before it was finished.
    pub truncated: bool,
    pub refusal: Option<Refusal>,
}

impl Assembly {
    pub fn refused(refusal: Refusal) -> Self {
        Self {
            start: [0; 3],
            pieces: Vec::new(),
            truncated: false,
            refusal: Some(refusal),
        }
    }

    pub fn count(&self, kind: PieceKind) -> usize {
        self.pieces
            .iter()
            .filter(|piece| piece.kind == kind)
            .count()
    }

    pub fn total(&self) -> usize {
        self.pieces.len()
    }

    /// The readout's one-line summary: `9 pieces (entrance 1, corridor 3, branch 3, stairs 1, room 2)`.
    pub fn summary(&self) -> String {
        let mut text = format!(
            "{} pieces (entrance {}, corridor {}, branch {}, stairs {}, room {}, ore deposit {}, zombie room {})",
            self.total(),
            self.count(PieceKind::Entrance),
            self.count(PieceKind::Corridor),
            self.count(PieceKind::Branch),
            self.count(PieceKind::Stairs),
            self.count(PieceKind::Room),
            self.count(PieceKind::OreDeposit),
            self.count(PieceKind::ZombieRoom),
        );
        if self.truncated {
            text.push_str(", truncated by the budget");
        }
        if let Some(refusal) = self.refusal {
            text.push_str(&format!(", refused {refusal:?}"));
        }
        text
    }
}

/// What the layout needs to know about the chunk it starts in.
#[derive(Clone, Copy, Debug)]
pub struct LayoutRequest {
    pub chunk: [i32; 2],
    /// The floor the caller drew (`findGenerationPoint`), already in the level's height range.
    pub floor_y: i32,
    /// The first free height above the chunk's centre column.
    pub surface_y: i32,
    pub facing: Facing,
    pub world_min_y: i32,
    pub world_max_y: i32,
    /// The most pieces this start may emit.
    pub budget: usize,
}

impl LayoutRequest {
    pub fn probe() -> Self {
        Self {
            chunk: PROBE_CHUNK,
            floor_y: PROBE_FLOOR_Y,
            surface_y: PROBE_SURFACE_Y,
            facing: Facing::South,
            world_min_y: -64,
            world_max_y: 320,
            budget: MAX_PIECES,
        }
    }
}

/// The chunk's start corner: upstream's `chunkPos.getBlockX(3)`, `getBlockZ(3)`.
pub fn chunk_origin(chunk: [i32; 2]) -> [i32; 3] {
    [chunk[0] * 16 + 3, 0, chunk[1] * 16 + 3]
}

/// The height a floor is clamped into: the level's own range, with room for the shaft's legs.
pub fn clamp_floor(floor_y: i32, world_min_y: i32, world_max_y: i32) -> i32 {
    let lowest = world_min_y + 6;
    let highest = world_max_y - 10;
    if highest < lowest {
        return lowest;
    }
    floor_y.clamp(lowest, highest)
}

/// The shaft's top: two blocks below the surface (`VerticalEntrance.determineDirection`'s
/// `ceilingHeight = minSurfaceHeight - 2`), never above the level's ceiling.
pub fn shaft_ceiling(surface_y: i32, world_max_y: i32) -> i32 {
    (surface_y - 2).min(world_max_y - 2)
}

/// A shaft must clear four blocks of climb, the ladder's length plus the doorway.
pub fn shaft_fits(floor_y: i32, ceiling_y: i32) -> bool {
    ceiling_y - floor_y >= 4
}

/// The bounds a piece with `dims` (secondary × height × length) occupies when its near face is at
/// `near` and it runs into `facing`. The width is centred on the near point, as upstream's
/// entrance places its 9-wide corridor on the 5-wide shaft's centre.
pub fn piece_box(near: [i32; 3], facing: Facing, dims: [i32; 3]) -> Box3 {
    let [width, height, length] = dims;
    let mut min = near;
    let mut max = near;
    max[1] = near[1] + height - 1;
    let main_axis = facing.axis();
    match facing {
        Facing::South | Facing::East => {
            min[main_axis] = near[main_axis];
            max[main_axis] = near[main_axis] + length - 1;
        }
        Facing::North | Facing::West => {
            min[main_axis] = near[main_axis] - length + 1;
            max[main_axis] = near[main_axis];
        }
    }
    let secondary = facing.right();
    let secondary_axis = if secondary[0] != 0 { 0 } else { 2 };
    let half = (width - 1) / 2;
    min[secondary_axis] = near[secondary_axis] - half;
    max[secondary_axis] = near[secondary_axis] + (width - 1 - half);
    Box3::new(min, max)
}

/// The entrance's shaft bounds: 5×5 centred on the start corner, from the floor to `ceiling_y`.
pub fn entrance_box(start: [i32; 3], floor_y: i32, ceiling_y: i32) -> Box3 {
    let half = (SHAFT_DIMS[0] - 1) / 2;
    Box3::new(
        [start[0] - half, floor_y, start[2] - half],
        [start[0] + half, ceiling_y, start[2] + half],
    )
}

/// Build a start's pieces: the entrance, the corridor chain, straight branch corridors and the
/// workstation rooms. Deterministic for the random source it is given; bounded by `budget`.
pub fn assemble<R: Roll>(random: &mut R, request: &LayoutRequest) -> Assembly {
    let mut assembly = Assembly {
        start: [0; 3],
        pieces: Vec::new(),
        truncated: false,
        refusal: None,
    };
    if request.budget < 2 {
        return Assembly::refused(Refusal::Budget {
            needed: 2,
            budget: request.budget,
        });
    }

    let origin = chunk_origin(request.chunk);
    let floor_y = clamp_floor(request.floor_y, request.world_min_y, request.world_max_y);
    if floor_y != request.floor_y {
        assembly.refusal = Some(Refusal::FloorClamped {
            asked: request.floor_y,
            clamped: floor_y,
        });
    }
    let ceiling_y = shaft_ceiling(request.surface_y, request.world_max_y);
    if !shaft_fits(floor_y, ceiling_y) {
        return Assembly {
            refusal: Some(Refusal::Shaft {
                floor: floor_y,
                ceiling: ceiling_y,
            }),
            ..assembly
        };
    }

    let start = [origin[0], floor_y, origin[2]];
    assembly.start = start;
    let entrance = PieceSpec {
        kind: PieceKind::Entrance,
        bounds: entrance_box(start, floor_y, ceiling_y),
        facing: request.facing,
        depth: -1,
        openings: Vec::new(),
        variant: 0,
    };
    push(&mut assembly, entrance, request.budget);

    // The corridor starts three blocks off the shaft's centre, as upstream places it, and runs in
    // the same direction; each segment's far face is the next segment's near face.
    let step = request.facing.step();
    let mut near = [start[0] + step[0] * 3, floor_y, start[2] + step[2] * 3];
    let mut depth = 0;
    loop {
        if depth > MAX_CORRIDOR_DEPTH {
            break;
        }
        let roll = random.next_i32(100);
        if !(roll >= 10 || depth < 2) {
            break;
        }
        let bounds = piece_box(near, request.facing, CORRIDOR_DIMS);
        let corridor = PieceSpec {
            kind: PieceKind::Corridor,
            bounds,
            facing: request.facing,
            depth,
            openings: Vec::new(),
            variant: 0,
        };
        let Some(corridor_index) = push(&mut assembly, corridor, request.budget) else {
            break;
        };

        // A branch hangs off one side wall, a run of at most three segments. Each segment draws
        // its kind the way `generateAndAddSmallTunnelPiece` does: `nextInt(100) >= 80` is a stair,
        // which takes the run four blocks lower and it continues there.
        if random.next_f32() < SMALL_SHAFT_SPAWN_RATE {
            let side = if random.next_i32(2) == 0 {
                right_of(request.facing)
            } else {
                left_of(request.facing)
            };
            let mut branch_floor = floor_y;
            let mut complete = true;
            let mut branch_near = offset_off_wall(&bounds, request.facing, side);
            let side_step = side.step();
            for chain in 0..MAX_BRANCH_SEGMENTS {
                let stairs = random.next_i32(100) >= STAIRS_ROLL_MIN;
                let dims = if stairs { STAIRS_DIMS } else { BRANCH_DIMS };
                let mut near = branch_near;
                near[1] = branch_floor;
                let branch_box = piece_box(near, side, dims);
                if chain == 0 {
                    let door = attached_opening(&bounds, request.facing, &branch_box, side, 1, 3);
                    if let Some(corridor) = assembly.pieces.get_mut(corridor_index) {
                        corridor.openings.push(door);
                    }
                }
                let branch = PieceSpec {
                    kind: if stairs {
                        PieceKind::Stairs
                    } else {
                        PieceKind::Branch
                    },
                    bounds: branch_box,
                    facing: side,
                    depth,
                    openings: Vec::new(),
                    variant: 0,
                };
                if push(&mut assembly, branch, request.budget).is_none() {
                    complete = false;
                    break;
                }
                branch_floor += if stairs { STAIRS_RISE } else { 0 };
                branch_near = [
                    near[0] + side_step[0] * dims[2],
                    branch_floor,
                    near[2] + side_step[2] * dims[2],
                ];
            }
            if complete {
                // The end of the run (`generateAndAddSmallTunnelPiece`'s last link): a zombie villager
                // room one time in fifty, else a pocket of ore.
                let (kind, dims, variant) = if random.next_i32(100) < ZOMBIE_ROOM_SPAWN_RATE {
                    (PieceKind::ZombieRoom, ZOMBIE_ROOM_DIMS, 0)
                } else {
                    let ore = ore_for_roll(random.next_i32(100));
                    (PieceKind::OreDeposit, ORE_DEPOSIT_DIMS, ore)
                };
                let end = PieceSpec {
                    kind,
                    bounds: piece_box(branch_near, side, dims),
                    facing: side,
                    depth,
                    openings: Vec::new(),
                    variant,
                };
                push(&mut assembly, end, request.budget);
            }
        }

        // The workstation room hangs off the other side wall, three blocks in from the segment's
        // near face (`buildSideRooms*`: `n + 9` of a 24-long piece).
        if random.next_f32() < WORKSTATION_SPAWN_RATE {
            let side = if random.next_i32(2) == 0 {
                left_of(request.facing)
            } else {
                right_of(request.facing)
            };
            let room_near = offset_off_wall_at(&bounds, request.facing, side, 9);
            let room_box = piece_box(room_near, side, ROOM_DIMS);
            let door = attached_opening(&bounds, request.facing, &room_box, side, 4, 2);
            if let Some(corridor) = assembly.pieces.get_mut(corridor_index) {
                corridor.openings.push(door);
            }
            let room = PieceSpec {
                kind: PieceKind::Room,
                bounds: room_box,
                facing: side,
                depth,
                openings: Vec::new(),
                variant: 0,
            };
            push(&mut assembly, room, request.budget);
        }

        // The far face is the box's high coordinate on a positive run and its low one on a
        // negative run; the next segment starts one block beyond it.
        let main_axis = request.facing.axis();
        near = match request.facing {
            Facing::South | Facing::East => {
                let mut next = near;
                next[main_axis] = bounds.max[main_axis] + 1;
                next
            }
            Facing::North | Facing::West => {
                let mut next = near;
                next[main_axis] = bounds.min[main_axis] - 1;
                next
            }
        };
        depth += 1;
    }
    assembly
}

/// Add a piece if the budget allows and it overlaps nothing; answer its index.
fn push(assembly: &mut Assembly, spec: PieceSpec, budget: usize) -> Option<usize> {
    if assembly.pieces.len() >= budget {
        assembly.truncated = true;
        return None;
    }
    if assembly
        .pieces
        .iter()
        .any(|existing| existing.bounds.intersects(&spec.bounds))
    {
        return None;
    }
    assembly.pieces.push(spec);
    Some(assembly.pieces.len() - 1)
}

/// The rail a stair step carries. Upstream draws a half chance per step and always sets
/// `ASCENDING_NORTH` (`SmallTunnelStairs.java:99-105`), which is the right slope for one facing of
/// four; the port's shape rises into the branch, the way its floor climbs, and the last step is
/// flat along the run.
pub fn stair_rail_shape(facing: Facing, last_step: bool) -> &'static str {
    if last_step {
        if facing.axis() == 2 {
            "north_south"
        } else {
            "east_west"
        }
    } else {
        match facing {
            Facing::North => "ascending_north",
            Facing::South => "ascending_south",
            Facing::East => "ascending_east",
            Facing::West => "ascending_west",
        }
    }
}

/// The opening a corridor carves so the piece hanging off its `side` wall can be entered. The
/// attached piece's secondary axis runs along the corridor's main axis; `offset`/`len` pick the
/// span of that axis its doorway covers.
fn attached_opening(
    corridor: &Box3,
    corridor_facing: Facing,
    attached: &Box3,
    side: Facing,
    offset: i32,
    len: i32,
) -> Opening {
    // The attached piece's secondary axis runs along the corridor's main axis.
    let along_axis = corridor_facing.axis();
    let first = attached.min[along_axis] + offset;
    Opening {
        along: corridor_along(corridor, corridor_facing, first),
        len,
        side,
    }
}

/// A world coordinate on a corridor's main axis, as the local index the corridor's writer uses.
fn corridor_along(corridor: &Box3, facing: Facing, world: i32) -> i32 {
    let axis = facing.axis();
    match facing {
        Facing::South | Facing::East => world - corridor.min[axis],
        Facing::North | Facing::West => corridor.max[axis] - world,
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

fn left_of(facing: Facing) -> Facing {
    right_of(facing).opposite()
}

/// A point one block outside the corridor's side wall, halfway along the segment.
fn offset_off_wall(bounds: &Box3, facing: Facing, side: Facing) -> [i32; 3] {
    offset_off_wall_at(bounds, facing, side, CORRIDOR_DIMS[2] / 2)
}

fn offset_off_wall_at(bounds: &Box3, facing: Facing, side: Facing, along: i32) -> [i32; 3] {
    let main_axis = facing.axis();
    let along_position = match facing {
        Facing::South | Facing::East => bounds.min[main_axis] + along,
        Facing::North | Facing::West => bounds.max[main_axis] - along,
    };
    let side_step = side.step();
    let secondary_axis = if side_step[0] != 0 { 0 } else { 2 };
    let wall = if side_step[secondary_axis] > 0 {
        bounds.max[secondary_axis] + 1
    } else {
        bounds.min[secondary_axis] - 1
    };
    let mut near = [0, bounds.min[1], 0];
    near[main_axis] = along_position;
    near[secondary_axis] = wall;
    near
}

/// A stable seed for one piece's decorations, from its bounds: the same piece lays the same supports
/// after a save/load and in every chunk pass that touches it.
pub fn piece_seed(bounds: &Box3) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for value in bounds.min.into_iter().chain(bounds.max) {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

/// `BigTunnel.buildSupports`: a big support when `nextInt(8)` is 0 or after ten blocks without
/// one, a small support when it is 1, and either skips three blocks. `(z, big)`.
pub fn corridor_supports(length: i32, seed: u64) -> Vec<(i32, bool)> {
    let mut random = VanillaRandom::new(seed as i64);
    let mut supports = Vec::new();
    let mut counter = 0;
    let mut z = 0;
    while z <= length - 2 {
        counter += 1;
        let roll = random.next_i32(8);
        if roll == 0 || counter >= 10 {
            supports.push((z, true));
            counter = 0;
            z += 4;
        } else if roll == 1 {
            supports.push((z, false));
            counter = 0;
            z += 4;
        } else {
            z += 1;
        }
    }
    supports
}

/// `SmallTunnel.buildSupports`: a support when `nextInt(7)` is 0, then five blocks further.
pub fn branch_supports(length: i32, seed: u64) -> Vec<i32> {
    let mut random = VanillaRandom::new(seed as i64);
    let mut supports = Vec::new();
    let mut z = 0;
    while z < length {
        if random.next_i32(7) == 0 {
            supports.push(z);
            z += 6;
        } else {
            z += 1;
        }
    }
    supports
}

/// `BigTunnel.buildGravelDeposits`: with `chance` over each z, a pile on the left when the draw is
/// below half the chance and on the right when it is below the whole one. `(z, right)`.
pub fn gravel_piles(length: i32, chance: f32, seed: u64) -> Vec<(i32, bool)> {
    let mut random = VanillaRandom::new(seed as i64);
    let mut piles = Vec::new();
    let mut z = 0;
    while z <= length - 2 {
        let roll = random.next_f32();
        if roll < chance / 2.0 {
            piles.push((z, false));
            z += 3;
        } else if roll < chance {
            piles.push((z, true));
            z += 3;
        } else {
            z += 1;
        }
    }
    piles
}

/// The deterministic readout's assembly: vanilla's LCG seeded on a fixed seed and chunk.
pub fn probe() -> Assembly {
    let mut random = VanillaRandom::new(0);
    random.set_large_feature_seed(PROBE_SEED, PROBE_CHUNK[0], PROBE_CHUNK[1]);
    let facing = random.facing();
    let mut request = LayoutRequest::probe();
    request.facing = facing;
    assemble(&mut random, &request)
}

/// The readout `/bettermineshafts status` prints: the structure, its biomes and the probe.
pub fn status_lines() -> Vec<String> {
    let assembly = probe();
    vec![
        format!("{MOD_ID}: structure {STRUCTURE_ID} (type {TYPE_ID}) — one of 13 carried variants",),
        format!(
            "{MOD_ID}: biomes {} required (+{OPTIONAL_BIOME_ENTRIES} optional): {}",
            BIOME_ENTRIES.len(),
            BIOME_ENTRIES.join(", "),
        ),
        format!(
            "{MOD_ID}: probe seed {PROBE_SEED} chunk {},{}, floor {PROBE_FLOOR_Y}, surface \
             {PROBE_SURFACE_Y} -> {}",
            PROBE_CHUNK[0],
            PROBE_CHUNK[1],
            assembly.summary(),
        ),
    ]
}

/// A block as the mod's own config names it: the canonical id and the properties it pins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockRef {
    pub name: String,
    pub properties: Vec<(String, String)>,
}

impl BlockRef {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            properties: Vec::new(),
        }
    }

    pub fn with(name: &str, properties: &[(&str, &str)]) -> Self {
        Self {
            name: name.to_owned(),
            properties: properties
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }

    pub fn property(&self, key: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }
}

/// One weighted alternative of a randomizer.
#[derive(Clone, Debug, PartialEq)]
pub struct Weighted {
    pub state: BlockRef,
    pub probability: f32,
}

/// YUNG's `BlockStateRandomizer`: a default state and the weighted states that may replace it.
#[derive(Clone, Debug, PartialEq)]
pub struct Randomizer {
    pub default: BlockRef,
    pub entries: Vec<Weighted>,
}

impl Randomizer {
    /// The state a `nextFloat` draw picks: the entries' cumulative probabilities, else the default.
    pub fn pick(&self, roll: f32) -> &BlockRef {
        let mut cumulative = 0.0;
        for entry in &self.entries {
            cumulative += entry.probability;
            if roll < cumulative {
                return &entry.state;
            }
        }
        &self.default
    }
}

/// Where the entrance's legs stand: on the corridor's edges or on its inner line. Upstream's
/// names are `edge` and `inner` (`BetterMineshaftConfiguration.LegVariant`), and the carried
/// configs use exactly those (`ice` and `mushroom` are `inner`); `center` is accepted as a legacy
/// alias because an earlier reading of the mod spelled it that way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegVariant {
    Edge,
    Inner,
}

impl LegVariant {
    pub fn name(self) -> &'static str {
        match self {
            Self::Edge => "edge",
            Self::Inner => "inner",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "edge" => Some(Self::Edge),
            "inner" | "center" => Some(Self::Inner),
            _ => None,
        }
    }
}

/// The biome-dependent decoration chances the config carries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecorationChances {
    pub vine: f32,
    pub snow: f32,
    pub cactus: f32,
    pub dead_bush: f32,
    pub mushroom: f32,
    pub gravel_pile: f32,
    pub lush: bool,
    pub dripstone: bool,
}

/// The eight named states a variant's palette is built from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockSet {
    pub main: BlockRef,
    pub support: BlockRef,
    pub slab: BlockRef,
    pub gravel: BlockRef,
    pub stone_wall: BlockRef,
    pub stone_slab: BlockRef,
    pub trapdoor: BlockRef,
    pub small_leg: BlockRef,
}

/// The four randomizers.
#[derive(Clone, Debug, PartialEq)]
pub struct RandomizerSet {
    pub main: Randomizer,
    pub floor: Randomizer,
    pub brick: Randomizer,
    pub leg: Randomizer,
}

/// One structure variant's whole palette, as `mineshaft_<variant>.json`'s `config` gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub replacement_rate: f32,
    pub leg_variant: LegVariant,
    pub decorations: DecorationChances,
    pub blocks: BlockSet,
    pub randomizers: RandomizerSet,
}

impl Palette {
    /// The oak variant, byte-for-byte the values in the carried
    /// `data/bettermineshafts/worldgen/structure/mineshaft_oak.json`. Used when a structure's
    /// config is missing or unreadable; the test beside it proves every name appears in that file.
    pub fn oak_defaults() -> Self {
        let planks = BlockRef::new("minecraft:oak_planks");
        Self {
            replacement_rate: 0.6,
            leg_variant: LegVariant::Edge,
            decorations: DecorationChances {
                vine: 0.1,
                snow: 0.0,
                cactus: 0.0,
                dead_bush: 0.0,
                mushroom: 0.0,
                gravel_pile: 0.1,
                lush: false,
                dripstone: false,
            },
            blocks: BlockSet {
                main: planks.clone(),
                support: BlockRef::new("minecraft:oak_fence"),
                slab: BlockRef::new("minecraft:oak_slab"),
                gravel: BlockRef::new("minecraft:gravel"),
                stone_wall: BlockRef::new("minecraft:stone_brick_wall"),
                stone_slab: BlockRef::new("minecraft:stone_brick_slab"),
                trapdoor: BlockRef::new("minecraft:oak_trapdoor"),
                small_leg: BlockRef::with("minecraft:stripped_oak_log", &[("axis", "y")]),
            },
            randomizers: RandomizerSet {
                main: Randomizer {
                    default: planks.clone(),
                    entries: vec![
                        Weighted {
                            state: BlockRef::new("minecraft:cobblestone"),
                            probability: 0.1,
                        },
                        Weighted {
                            state: BlockRef::new("minecraft:stone_bricks"),
                            probability: 0.1,
                        },
                        Weighted {
                            state: BlockRef::new("minecraft:mossy_stone_bricks"),
                            probability: 0.1,
                        },
                        Weighted {
                            state: BlockRef::new("minecraft:cracked_stone_bricks"),
                            probability: 0.1,
                        },
                        Weighted {
                            state: BlockRef::new("minecraft:cave_air"),
                            probability: 0.2,
                        },
                    ],
                },
                floor: Randomizer {
                    default: planks,
                    entries: vec![
                        Weighted {
                            state: BlockRef::new("minecraft:cobblestone"),
                            probability: 0.1,
                        },
                        Weighted {
                            state: BlockRef::new("minecraft:stone_bricks"),
                            probability: 0.1,
                        },
                        Weighted {
                            state: BlockRef::new("minecraft:mossy_stone_bricks"),
                            probability: 0.1,
                        },
                        Weighted {
                            state: BlockRef::new("minecraft:cracked_stone_bricks"),
                            probability: 0.1,
                        },
                    ],
                },
                brick: Randomizer {
                    default: BlockRef::new("minecraft:stone_bricks"),
                    entries: vec![
                        Weighted {
                            state: BlockRef::new("minecraft:stone_bricks"),
                            probability: 0.333,
                        },
                        Weighted {
                            state: BlockRef::new("minecraft:mossy_stone_bricks"),
                            probability: 0.333,
                        },
                        Weighted {
                            state: BlockRef::new("minecraft:cracked_stone_bricks"),
                            probability: 0.333,
                        },
                    ],
                },
                leg: Randomizer {
                    default: BlockRef::with("minecraft:stripped_oak_log", &[("axis", "y")]),
                    entries: Vec::new(),
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic source for the tests: splitmix64, so a test can name its seed.
    struct TestRandom(u64);

    impl TestRandom {
        fn new(seed: u64) -> Self {
            Self(seed)
        }

        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
    }

    impl Roll for TestRandom {
        fn next_i32(&mut self, bound: i32) -> i32 {
            if bound <= 0 {
                0
            } else {
                (self.next() % bound as u64) as i32
            }
        }

        fn next_f32(&mut self) -> f32 {
            (self.next() >> 40) as f32 / (1_u64 << 24) as f32
        }
    }

    /// A start built the way the game builds one: vanilla's LCG seeded on a world seed and chunk.
    fn assemble_with(seed: i64) -> Assembly {
        let mut random = VanillaRandom::new(0);
        random.set_large_feature_seed(seed, 0, 0);
        let facing = random.facing();
        let request = LayoutRequest {
            facing,
            ..request(facing)
        };
        assemble(&mut random, &request)
    }

    fn request(facing: Facing) -> LayoutRequest {
        LayoutRequest {
            chunk: [0, 0],
            floor_y: -20,
            surface_y: 64,
            facing,
            world_min_y: -64,
            world_max_y: 320,
            budget: MAX_PIECES,
        }
    }

    #[test]
    fn a_normal_start_places_an_entrance_a_corridor_a_branch_and_a_room() {
        // Seed 320 places all five kinds; the pinned summary is what the readout reports.
        let assembly = assemble_with(320);
        assert!(assembly.refusal.is_none());
        assert_eq!(assembly.count(PieceKind::Entrance), 1);
        assert_eq!(assembly.count(PieceKind::Corridor), 4);
        assert_eq!(assembly.count(PieceKind::Branch), 2);
        assert_eq!(assembly.count(PieceKind::Stairs), 1);
        assert_eq!(assembly.count(PieceKind::Room), 1);
        assert_eq!(assembly.count(PieceKind::OreDeposit), 1);
        assert_eq!(
            assembly.summary(),
            "10 pieces (entrance 1, corridor 4, branch 2, stairs 1, room 1, ore deposit 1, zombie room 0)"
        );
        assert!(!assembly.truncated);
        assert!(assembly.total() <= MAX_PIECES);
        // The start is the chunk's block (3, floor, 3): upstream's `getBlockX(3)`.
        assert_eq!(assembly.start, [3, -20, 3]);
        // The corridor chain never exceeds upstream's `chainLength > 3` stop.
        let longest = assembly
            .pieces
            .iter()
            .filter(|piece| piece.kind == PieceKind::Corridor)
            .map(|piece| piece.depth)
            .max()
            .unwrap_or(-1);
        assert!(longest <= MAX_CORRIDOR_DEPTH, "{longest}");
    }

    #[test]
    fn the_same_seed_gives_the_same_assembly_and_a_different_seed_does_not() {
        assert_eq!(assemble_with(0), assemble_with(0));
        assert_eq!(assemble_with(7), assemble_with(7));
        assert_ne!(assemble_with(0).pieces, assemble_with(7).pieces);
    }

    #[test]
    fn pieces_do_not_overlap_and_their_local_frames_agree_with_their_boxes() {
        for seed in [0, 1, 5, 18] {
            let assembly = assemble_with(seed);
            check_boxes(&assembly);
        }
    }

    fn check_boxes(assembly: &Assembly) {
        for (index, piece) in assembly.pieces.iter().enumerate() {
            for other in &assembly.pieces[index + 1..] {
                assert!(
                    !piece.bounds.intersects(&other.bounds),
                    "{} {:?} overlaps {} {:?}",
                    piece.kind.name(),
                    piece.bounds,
                    other.kind.name(),
                    other.bounds
                );
            }
            let (box_from_spec, corners) = piece.corners();
            assert_eq!(box_from_spec, piece.bounds);
            assert_eq!(
                corners,
                piece.bounds,
                "local frame of {}",
                piece.kind.name()
            );
        }
    }

    #[test]
    fn every_facing_builds_a_contiguous_corridor_chain() {
        // A negative run (west/north) reads its far face from the box's low coordinate; getting
        // that wrong makes the second segment overlap the first, so the chain stops after one.
        let mut facings = std::collections::BTreeSet::new();
        for seed in 0..40 {
            let assembly = assemble_with(seed);
            facings.insert(assembly.pieces[0].facing.name());
            let corridors: Vec<&PieceSpec> = assembly
                .pieces
                .iter()
                .filter(|piece| piece.kind == PieceKind::Corridor)
                .collect();
            assert!(
                corridors.len() >= 2,
                "seed {seed} built no chain: {}",
                assembly.summary()
            );
            for pair in corridors.windows(2) {
                let (previous, next) = (pair[0], pair[1]);
                let axis = previous.facing.axis();
                // The near face is the box's low coordinate on a positive run and its high one
                // on a negative run; the next segment starts one block beyond the previous far
                // face.
                match previous.facing {
                    Facing::South | Facing::East => assert_eq!(
                        next.bounds.min[axis],
                        previous.bounds.max[axis] + 1,
                        "seed {seed} breaks the chain"
                    ),
                    Facing::North | Facing::West => assert_eq!(
                        next.bounds.max[axis],
                        previous.bounds.min[axis] - 1,
                        "seed {seed} breaks the chain"
                    ),
                }
            }
        }
        assert_eq!(
            facings.len(),
            4,
            "the sample covers all four facings: {facings:?}"
        );
    }

    #[test]
    fn a_segment_continues_where_the_previous_one_ended() {
        let assembly = assemble_with(0);
        let corridors: Vec<&PieceSpec> = assembly
            .pieces
            .iter()
            .filter(|piece| piece.kind == PieceKind::Corridor)
            .collect();
        for pair in corridors.windows(2) {
            let (previous, next) = (pair[0], pair[1]);
            let axis = previous.facing.axis();
            let expected = match previous.facing {
                Facing::South | Facing::East => previous.bounds.max[axis] + 1,
                Facing::North | Facing::West => previous.bounds.min[axis] - 1,
            };
            assert_eq!(
                next.bounds.min[axis]
                    .min(next.bounds.max[axis])
                    .max(next.bounds.min[axis]),
                next.bounds.min[axis]
            );
            assert!(
                next.bounds.min[axis] == expected || next.bounds.max[axis] == expected,
                "{:?} then {:?} along axis {axis}",
                previous.bounds,
                next.bounds
            );
            assert_eq!(next.bounds.min[1], previous.bounds.min[1]);
        }
    }

    #[test]
    fn the_entrance_shaft_is_five_wide_and_centred_on_the_start() {
        let assembly = assemble_with(0);
        let entrance = &assembly.pieces[0];
        assert_eq!(entrance.kind, PieceKind::Entrance);
        // The surface is 64, so the shaft's ceiling is 62 and its floor -20: 83 blocks.
        assert_eq!(entrance.bounds.span(), [5, 83, 5]);
        assert_eq!(entrance.bounds.min[0], assembly.start[0] - 2);
        assert_eq!(entrance.bounds.min[2], assembly.start[2] - 2);
        assert_eq!(entrance.bounds.max[1], shaft_ceiling(64, 320));
    }

    #[test]
    fn a_surface_above_the_world_ceiling_is_clamped_and_the_shaft_still_fits() {
        // The clamp case: the surface reports 400 in a 320-tall level.
        let request = LayoutRequest {
            floor_y: -55,
            surface_y: 400,
            ..request(Facing::North)
        };
        let assembly = assemble(&mut TestRandom::new(3), &request);
        assert!(assembly.refusal.is_none());
        let ceiling = shaft_ceiling(400, 320);
        assert_eq!(ceiling, 318);
        assert_eq!(assembly.pieces[0].bounds.max[1], 318);
        assert!(assembly.pieces[0].bounds.span()[1] <= 320 - (-64));
    }

    #[test]
    fn a_floor_under_the_world_floor_is_clamped_and_recorded() {
        let request = LayoutRequest {
            floor_y: -300,
            ..request(Facing::East)
        };
        let assembly = assemble(&mut TestRandom::new(5), &request);
        assert_eq!(
            assembly.refusal,
            Some(Refusal::FloorClamped {
                asked: -300,
                clamped: -58
            })
        );
        assert_eq!(assembly.start[1], -58);
    }

    #[test]
    fn a_surface_too_low_over_the_floor_refuses_the_start() {
        let request = LayoutRequest {
            floor_y: 10,
            surface_y: 12,
            ..request(Facing::South)
        };
        let assembly = assemble(&mut TestRandom::new(1), &request);
        assert_eq!(
            assembly.refusal,
            Some(Refusal::Shaft {
                floor: 10,
                ceiling: 10
            })
        );
        assert!(assembly.pieces.is_empty());
    }

    #[test]
    fn a_budget_below_two_refuses_the_start() {
        let request = LayoutRequest {
            budget: 1,
            ..request(Facing::South)
        };
        let assembly = assemble(&mut TestRandom::new(1), &request);
        assert_eq!(
            assembly.refusal,
            Some(Refusal::Budget {
                needed: 2,
                budget: 1
            })
        );
        assert!(assembly.pieces.is_empty());
        assert!(!assembly.truncated);
    }

    #[test]
    fn a_small_budget_stops_the_chain_and_says_so() {
        let funded_less = LayoutRequest {
            budget: 3,
            ..request(Facing::South)
        };
        let assembly = assemble(&mut TestRandom::new(7), &funded_less);
        assert_eq!(assembly.total(), 3);
        assert!(assembly.truncated, "{}", assembly.summary());
        // The unfunded assembly is a prefix of the funded one: the budget stops a chain, it does
        // not change the chain.
        let full = assemble(&mut TestRandom::new(7), &request(Facing::South));
        assert_eq!(assembly.pieces[..], full.pieces[..3]);
    }

    #[test]
    fn the_probe_is_deterministic_and_its_summary_is_what_the_readout_prints() {
        let first = probe();
        let again = probe();
        assert_eq!(first, again);
        assert!(first.refusal.is_none());
        assert_eq!(
            first.summary(),
            "9 pieces (entrance 1, corridor 3, branch 2, stairs 1, room 1, ore deposit 1, zombie room 0)"
        );
        let lines = status_lines();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains(STRUCTURE_ID));
        assert!(lines[0].contains(TYPE_ID));
        assert!(lines[1].contains("biomes 4 required (+3 optional)"));
        for biome in BIOME_ENTRIES {
            assert!(lines[1].contains(biome));
        }
        assert!(lines[2].contains(&first.summary()));
        assert!(
            lines[2].contains("stairs 1"),
            "the readout names the stair kind"
        );
    }

    #[test]
    fn the_biome_list_is_the_carried_tag_and_the_palette_is_the_carried_config() {
        let tag = include_str!(
            "../data/bettermineshafts/tags/worldgen/biome/has_structure/better_mineshaft_oak.json"
        );
        for biome in BIOME_ENTRIES {
            assert!(
                tag.contains(&format!("\"{biome}\"")),
                "{biome} missing from the tag"
            );
        }
        let optional = tag.matches("\"required\": false").count();
        assert_eq!(optional, OPTIONAL_BIOME_ENTRIES);

        let config = include_str!("../data/bettermineshafts/worldgen/structure/mineshaft_oak.json");
        let palette = Palette::oak_defaults();
        let mut names = vec![
            &palette.blocks.main.name,
            &palette.blocks.support.name,
            &palette.blocks.slab.name,
            &palette.blocks.gravel.name,
            &palette.blocks.stone_wall.name,
            &palette.blocks.stone_slab.name,
            &palette.blocks.trapdoor.name,
            &palette.blocks.small_leg.name,
        ];
        names.extend(
            palette
                .randomizers
                .main
                .entries
                .iter()
                .map(|w| &w.state.name),
        );
        names.extend(
            palette
                .randomizers
                .floor
                .entries
                .iter()
                .map(|w| &w.state.name),
        );
        names.extend(
            palette
                .randomizers
                .brick
                .entries
                .iter()
                .map(|w| &w.state.name),
        );
        for name in names {
            assert!(config.contains(name), "{name} is not in the carried config");
        }
        assert!(config.contains("\"replacementRate\": 0.6"));
        assert!(config.contains("\"legVariant\": \"edge\""));
        assert!(config.contains("\"vineChance\": 0.1"));
        assert!(config.contains("\"gravelPileChance\": 0.1"));
    }

    #[test]
    fn the_randomizer_picks_by_cumulative_probability_and_falls_back_to_its_default() {
        let randomizer = Palette::oak_defaults().randomizers.main;
        assert_eq!(randomizer.pick(0.05).name, "minecraft:cobblestone");
        assert_eq!(randomizer.pick(0.15).name, "minecraft:stone_bricks");
        assert_eq!(randomizer.pick(0.35).name, "minecraft:cracked_stone_bricks");
        assert_eq!(randomizer.pick(0.55).name, "minecraft:cave_air");
        // The entries sum to 0.6: everything above is the default state.
        assert_eq!(randomizer.pick(0.95).name, "minecraft:oak_planks");
    }

    #[test]
    fn vanilla_random_matches_the_lcg_arithmetic_it_documents() {
        let mut random = VanillaRandom::new(0);
        random.set_large_feature_seed(0, 0, 0);
        // The first advance from seed 0 is the LCG's increment, right-shifted by 33 bits.
        assert_eq!(
            random.next_i32(4),
            (1_442_695_040_888_963_407_i64 >> 33) as i32 % 4
        );
        let mut seeded = VanillaRandom::new(0);
        seeded.set_large_feature_seed(42, -3, 7);
        let expected = 42 ^ (-3_i64 * 3_418_731_287_142) ^ (7_i64 * 132_897_987_541);
        assert_eq!(seeded, VanillaRandom::new(expected));
    }

    #[test]
    fn decorations_are_stable_and_inside_their_piece() {
        let seed = piece_seed(&Box3::new([0, 0, 0], [8, 7, 23]));
        assert_eq!(seed, piece_seed(&Box3::new([0, 0, 0], [8, 7, 23])));
        let supports = corridor_supports(CORRIDOR_DIMS[2], seed);
        assert!(!supports.is_empty());
        assert_eq!(supports, corridor_supports(CORRIDOR_DIMS[2], seed));
        let mut previous = -4;
        for (z, _) in &supports {
            assert!(*z >= 0 && *z <= CORRIDOR_DIMS[2] - 2, "{z}");
            assert!(*z > previous, "{supports:?}");
            previous = *z;
        }
        let piles = gravel_piles(CORRIDOR_DIMS[2], 0.1, seed);
        for (z, right) in &piles {
            assert!(*z >= 0 && *z <= CORRIDOR_DIMS[2] - 2);
            let _ = right;
        }
        let branch = branch_supports(BRANCH_DIMS[2], seed);
        for z in &branch {
            assert!(*z < BRANCH_DIMS[2], "{z}");
        }
    }

    #[test]
    fn a_corridor_opens_its_wall_where_a_room_hangs_off_it() {
        let assembly = assemble_with(320);
        let room = assembly
            .pieces
            .iter()
            .find(|piece| piece.kind == PieceKind::Room)
            .expect("seed 320 places a room");
        let corridor = assembly
            .pieces
            .iter()
            .find(|piece| piece.kind == PieceKind::Corridor && !piece.openings.is_empty())
            .expect("a corridor carries the room's opening");
        let opening = corridor
            .openings
            .iter()
            .find(|opening| opening.side == room.facing)
            .expect("an opening on the room's side");
        assert_eq!(opening.side, room.facing);
        assert_eq!(opening.len, 2);
        let axis = corridor.facing.axis();
        let world = match corridor.facing {
            Facing::South | Facing::East => corridor.bounds.min[axis] + opening.along,
            Facing::North | Facing::West => corridor.bounds.max[axis] - opening.along,
        };
        // The room's doorway is its local x 4..=5, its secondary axis running along the corridor.
        assert_eq!(world, room.bounds.min[axis] + 4);
        // And the opening is inside the corridor's own length.
        assert!((0..CORRIDOR_DIMS[2] - 1).contains(&opening.along));
    }

    #[test]
    fn facings_round_trip_and_their_axes_are_right_handed() {
        for facing in [Facing::North, Facing::South, Facing::East, Facing::West] {
            assert_eq!(Facing::from_name(facing.name()), Some(facing));
            assert_eq!(facing.opposite().opposite(), facing);
            let step = facing.step();
            let right = facing.right();
            assert_eq!(step[1], 0);
            assert_eq!(right[1], 0);
            // The main axis and the secondary axis differ, and only one component is non-zero.
            assert_ne!(facing.axis(), if right[0] != 0 { 0 } else { 2 });
            assert_eq!(step.iter().filter(|value| **value != 0).count(), 1);
        }
        assert_eq!(Facing::from_roll(0), Facing::South);
        assert_eq!(Facing::from_roll(2), Facing::North);
    }

    #[test]
    fn a_stair_piece_is_nine_high_and_the_run_continues_four_blocks_above_it() {
        // A branch segment may be a stair (`BetterMineshaftGenerator:71-78`): 5 wide, 9 high, 5
        // long, and the run resumes four blocks higher (`SmallTunnelStairs.addChildren` places the
        // next piece at `minY + 4`).
        let mut checked = 0;
        for seed in 0..120 {
            let assembly = assemble_with(seed);
            for (index, piece) in assembly.pieces.iter().enumerate() {
                if piece.kind != PieceKind::Stairs {
                    continue;
                }
                checked += 1;
                assert_eq!(piece.bounds.span(), STAIRS_DIMS, "seed {seed}");
                let Some(next) = assembly.pieces.get(index + 1) else {
                    continue;
                };
                if next.kind != PieceKind::Branch && next.kind != PieceKind::Stairs {
                    continue;
                }
                assert_eq!(
                    next.bounds.min[1],
                    piece.bounds.min[1] + 4,
                    "seed {seed}: the run after a stair continues four blocks higher"
                );
            }
        }
        assert!(checked > 0, "no seed in the sample placed a stair");
    }

    #[test]
    fn every_branch_run_is_contiguous_and_its_stairs_step_up_four() {
        // The run's pieces are adjacent (one block past the far face) and only a stair moves it:
        // a segment after a stair must sit four blocks above the stair's floor.
        let mut stairs_seen = 0;
        for seed in 0..120 {
            let assembly = assemble_with(seed);
            let mut previous: Option<&PieceSpec> = None;
            for piece in &assembly.pieces {
                let in_run = piece.kind == PieceKind::Branch || piece.kind == PieceKind::Stairs;
                if !in_run {
                    previous = None;
                    continue;
                }
                if piece.kind == PieceKind::Stairs {
                    stairs_seen += 1;
                }
                if let Some(previous) = previous {
                    let axis = previous.facing.axis();
                    let expected = match previous.facing {
                        Facing::South | Facing::East => previous.bounds.max[axis] + 1,
                        Facing::North | Facing::West => previous.bounds.min[axis] - 1,
                    };
                    let reached = if matches!(previous.facing, Facing::South | Facing::East) {
                        piece.bounds.min[axis]
                    } else {
                        piece.bounds.max[axis]
                    };
                    assert_eq!(reached, expected, "seed {seed}: the run is not contiguous");
                    let rise = if previous.kind == PieceKind::Stairs {
                        4
                    } else {
                        0
                    };
                    assert_eq!(
                        piece.bounds.min[1],
                        previous.bounds.min[1] + rise,
                        "seed {seed}: a segment after a stair must sit four higher"
                    );
                }
                previous = Some(piece);
            }
        }
        assert!(stairs_seen > 0, "no seed in the sample placed a stair");
    }

    #[test]
    fn the_stair_rails_rise_into_the_branch_on_every_facing() {
        // The stair's floor climbs with the run (`SmallTunnelStairs.postProcess`: the floor at
        // local z is local y = z), so its rails must rise the same way. Upstream sets
        // ASCENDING_NORTH on every step (`:100-105`) whatever the piece's direction, which is the
        // right slope for one facing of four; the port's shape follows the piece, and the last
        // step is flat along the run.
        for facing in [Facing::South, Facing::West, Facing::North, Facing::East] {
            for _step in 0..STAIRS_DIMS[2] - 1 {
                assert_eq!(
                    stair_rail_shape(facing, false),
                    format!("ascending_{}", facing.name()),
                    "facing {}",
                    facing.name()
                );
            }
            let flat = if facing.axis() == 2 {
                "north_south"
            } else {
                "east_west"
            };
            assert_eq!(stair_rail_shape(facing, true), flat);
        }
        assert_ne!(
            stair_rail_shape(Facing::South, false),
            "ascending_north",
            "a south-running stair rises toward the south"
        );
    }

    #[test]
    fn vanilla_random_is_never_negative_and_stays_inside_its_bounds() {
        // Vanilla's `next(bits)` is `(int)(seed >>> (64 - bits))` — an UNSIGNED shift. A signed one
        // returns a negative value for every seed with the top bit set, which makes `nextInt(100)`
        // a negative roll (the corridor chain stops at two segments) and `nextFloat()` negative (a
        // branch on most segments). Both are wrong, and the readout is built on this source.
        let mut random = VanillaRandom::new(0);
        random.set_large_feature_seed(12_345, -7, 11);
        let mut rolls = [0_usize; 4];
        for _ in 0..400 {
            let value = random.next_i32(100);
            assert!((0..100).contains(&value), "nextInt(100) returned {value}");
            let roll = random.next_f32();
            assert!((0.0..1.0).contains(&roll), "nextFloat returned {roll}");
            rolls[random.next_i32(4) as usize] += 1;
        }
        assert!(
            rolls.iter().all(|count| *count > 40),
            "every facing is drawn: {rolls:?}"
        );
    }

    #[test]
    fn a_saved_box_span_is_checked_before_it_is_iterated() {
        assert_eq!(
            Box3::new([0, 0, 0], [8, 7, 23]).span_checked(),
            Some([9, 8, 24])
        );
        assert!(
            Box3::new([0, 0, 0], [8, 7, 23]).is_plausible(),
            "the corridor's own box"
        );
        assert_eq!(
            Box3::new([1, 0, 0], [0, 0, 0]).span_checked(),
            None,
            "an inverted axis"
        );
        assert_eq!(
            Box3::new([0, 0, 0], [i32::MAX, 0, 0]).span_checked(),
            None,
            "a span that does not fit an i32"
        );
        assert!(
            !Box3::new([0, 0, 0], [1_000_000, 0, 0]).is_plausible(),
            "an attacker-sized axis"
        );
    }

    #[test]
    fn ore_rolls_follow_the_default_weights() {
        // 50 cobble, 20 coal, 9 iron, 7 redstone, 7 gold, 3 lapis, 3 emerald, 1 diamond.
        let picks: Vec<i32> = [0, 49, 50, 69, 70, 78, 79, 85, 86, 92, 93, 95, 96, 98, 99]
            .iter()
            .map(|roll| ore_for_roll(*roll))
            .collect();
        assert_eq!(picks, [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7]);
        assert_eq!(ORE_WEIGHTS.iter().sum::<i32>(), 100);
    }

    #[test]
    fn a_branch_ends_in_an_ore_deposit_or_a_zombie_room() {
        let assembly = assemble_with(320);
        let end = assembly
            .pieces
            .iter()
            .find(|piece| piece.kind == PieceKind::OreDeposit)
            .expect("seed 320 ends a branch in ore");
        assert_eq!(end.bounds.span(), ORE_DEPOSIT_DIMS);
        let last_branch = assembly
            .pieces
            .iter()
            .rfind(|piece| matches!(piece.kind, PieceKind::Branch | PieceKind::Stairs))
            .expect("a branch run");
        assert_eq!(last_branch.facing, end.facing);
    }
}
