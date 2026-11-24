//! The glue: the structure type, its saved-piece loader, the config reader and the command.
//!
//! Every decision is [`crate::runtime`]'s; this file only hands values to the SDK and asks the
//! contexts it is given. The config reader is deliberately tolerant: the carried structure JSON's
//! `config` arrives as NBT (numbers may be int, long, float or double; ids are `id` in the 26.3
//! shape and `Name` in the 1.21.1 one), and a field the reader cannot see keeps the oak default
//! rather than failing the whole structure.

use std::sync::{Arc, RwLock};

use mod_api::{
    self, BlockMetadata, Command, CommandRegistration, Compound, FoundStructure, GenerationContext,
    HeightmapType, Host, RandomSource, Registrar, ServerHost, SettingSpec, StructureError,
    StructurePiece, StructureType, Tag, Text,
};

use crate::pieces::{self, Piece};
use crate::runtime::{
    self, BlockRef, DecorationChances, LayoutRequest, LegVariant, Palette, Randomizer, Roll,
    Settings, Weighted,
};

/// The type the jar's data-pack structures name: `"type": "bettermineshafts:mineshaft"`.
pub struct MineshaftType;

impl StructureType for MineshaftType {
    /// `BetterMineshaftStructure.findGenerationPoint` with `generatePieces`: one height draw (the
    /// upstream class draws it twice — see `runtime`'s note), then the entrance, the corridor chain
    /// and what hangs off it.
    fn find_generation_point(&self, ctx: &mut dyn GenerationContext) -> Option<FoundStructure> {
        let chunk = ctx.chunk();
        let chunk_xz = [chunk.x, chunk.z];
        let seed = ctx.seed();
        let (world_min_y, world_max_y) = (ctx.min_y(), ctx.max_y());

        // Upstream's first draw, then the floor inside the config's range (`-55..=30`).
        let settings = settings();
        ctx.random().next_double();
        let range = settings.max_y - settings.min_y + 1;
        if range < 1 {
            // `nextInt` refuses a bound below one: the original throws, so no mineshaft starts.
            return None;
        }
        let drawn = ctx.random().next_int_bound(range) + settings.min_y;

        // The direction comes from the same chunk-seeded LCG upstream uses.
        let mut direction = runtime::VanillaRandom::new(0);
        direction.set_large_feature_seed(seed, chunk.x, chunk.z);
        let facing = direction.facing();

        let palette = palette_from(&ctx.config());
        let blocks = Arc::new(pieces::resolve(ctx.metadata(), &palette).ok()?);
        let request = LayoutRequest {
            chunk: chunk_xz,
            floor_y: drawn,
            facing,
            world_min_y,
            world_max_y,
            budget: runtime::MAX_PIECES,
            settings,
        };
        let mut assembly = {
            let mut random = EngineRoll(ctx.random());
            runtime::assemble(&mut random, &request)
        };
        if assembly.pieces.is_empty() {
            return None;
        }
        // The entrance's shaft and tunnel, decided once for every chunk it spans. The generator's
        // first free height less one is `SurfaceHelper.getSurfaceHeight`'s topmost non-air block.
        for spec in &mut assembly.pieces {
            if spec.kind == runtime::PieceKind::Entrance {
                spec.entrance = runtime::entrance_site(spec, |x, z| {
                    ctx.base_height(x, z, HeightmapType::WorldSurfaceWg) - 1
                });
            }
        }
        // What the readout's `last` answers, so a game run can find a rare piece (a stair) exactly.
        record_start(chunk_xz, &assembly);
        let placed: Vec<Arc<dyn StructurePiece>> = assembly
            .pieces
            .iter()
            .map(|spec| {
                Arc::new(Piece::new(
                    spec.clone(),
                    palette.clone(),
                    blocks.clone(),
                    settings,
                )) as Arc<dyn StructurePiece>
            })
            .collect();
        Some(FoundStructure {
            position: assembly.start,
            pieces: placed,
            ..Default::default()
        })
    }

    /// `StructurePieceType.load`: a saved piece is rebuilt from its own tag, never regenerated.
    fn load_piece(
        &self,
        metadata: &dyn BlockMetadata,
        tag: &Compound,
    ) -> Result<Arc<dyn StructurePiece>, StructureError> {
        let (spec, palette) = pieces::piece_from_tag(tag)?;
        let blocks = Arc::new(pieces::resolve(metadata, &palette)?);
        Ok(Arc::new(Piece::new(spec, palette, blocks, settings())))
    }
}

/// The game's piece random as the layout's [`Roll`].
struct EngineRoll<'a>(&'a mut dyn RandomSource);

impl Roll for EngineRoll<'_> {
    fn next_i32(&mut self, bound: i32) -> i32 {
        self.0.next_int_bound(bound)
    }

    fn next_f32(&mut self) -> f32 {
        self.0.next_float()
    }
}

/// The block-key table the registrar filled at startup: every name this port can place.
static KEYS: std::sync::OnceLock<std::collections::BTreeMap<&'static str, mod_api::BlockKey>> =
    std::sync::OnceLock::new();

/// Every block the port may place: the fixed set the piece writers name, plus every block id the
/// thirteen carried structure configs name (all palettes, so a variant's own palette resolves).
/// A test beside this table reads the carried files and fails if one names a block it lacks.
pub const REQUESTED_BLOCKS: [&str; 106] = [
    "minecraft:wall_torch",
    "minecraft:acacia_fence",
    "minecraft:cactus",
    "minecraft:brown_mushroom",
    "minecraft:dead_bush",
    "minecraft:dirt",
    "minecraft:moss_carpet",
    "minecraft:red_mushroom",
    "minecraft:snow",
    "minecraft:terracotta",
    "minecraft:chest",
    "minecraft:furnace",
    "minecraft:anvil",
    "minecraft:barrel",
    "minecraft:black_bed",
    "minecraft:blast_furnace",
    "minecraft:coal_ore",
    "minecraft:cobblestone_stairs",
    "minecraft:diamond_ore",
    "minecraft:emerald_ore",
    "minecraft:gold_ore",
    "minecraft:iron_door",
    "minecraft:iron_ore",
    "minecraft:lapis_ore",
    "minecraft:redstone_ore",
    "minecraft:redstone_torch",
    "minecraft:smithing_table",
    "minecraft:spawner",
    "minecraft:stone",
    "minecraft:stone_button",
    "minecraft:stone_slab",
    "minecraft:acacia_planks",
    "minecraft:acacia_slab",
    "minecraft:acacia_trapdoor",
    "minecraft:air",
    "minecraft:blue_ice",
    "minecraft:brown_mushroom_block",
    "minecraft:brown_terracotta",
    "minecraft:cave_air",
    "minecraft:chiseled_red_sandstone",
    "minecraft:chiseled_sandstone",
    "minecraft:chiseled_stone_bricks",
    "minecraft:cobblestone",
    "minecraft:cobblestone_slab",
    "minecraft:cobblestone_wall",
    "minecraft:cobweb",
    "minecraft:crafting_table",
    "minecraft:cracked_stone_bricks",
    "minecraft:cut_red_sandstone",
    "minecraft:cut_sandstone",
    "minecraft:dark_oak_fence",
    "minecraft:dark_oak_planks",
    "minecraft:dark_oak_slab",
    "minecraft:dark_oak_trapdoor",
    "minecraft:gravel",
    "minecraft:iron_bars",
    "minecraft:jungle_fence",
    "minecraft:jungle_planks",
    "minecraft:jungle_slab",
    "minecraft:jungle_trapdoor",
    "minecraft:ladder",
    "minecraft:lantern",
    "minecraft:moss_block",
    "minecraft:mossy_cobblestone",
    "minecraft:mossy_cobblestone_slab",
    "minecraft:mossy_cobblestone_wall",
    "minecraft:mossy_stone_bricks",
    "minecraft:mushroom_stem",
    "minecraft:mycelium",
    "minecraft:oak_fence",
    "minecraft:oak_log",
    "minecraft:oak_planks",
    "minecraft:oak_slab",
    "minecraft:oak_trapdoor",
    "minecraft:orange_terracotta",
    "minecraft:packed_ice",
    "minecraft:powered_rail",
    "minecraft:rail",
    "minecraft:red_mushroom_block",
    "minecraft:red_sand",
    "minecraft:red_sandstone",
    "minecraft:red_sandstone_slab",
    "minecraft:red_sandstone_wall",
    "minecraft:sand",
    "minecraft:sandstone",
    "minecraft:sandstone_slab",
    "minecraft:sandstone_wall",
    "minecraft:smooth_red_sandstone",
    "minecraft:smooth_sandstone",
    "minecraft:snow_block",
    "minecraft:spruce_fence",
    "minecraft:spruce_leaves",
    "minecraft:spruce_planks",
    "minecraft:spruce_slab",
    "minecraft:spruce_trapdoor",
    "minecraft:stone_brick_slab",
    "minecraft:stone_brick_wall",
    "minecraft:stone_bricks",
    "minecraft:stripped_acacia_log",
    "minecraft:stripped_dark_oak_log",
    "minecraft:stripped_jungle_log",
    "minecraft:stripped_oak_log",
    "minecraft:stripped_spruce_log",
    "minecraft:vine",
    "minecraft:white_terracotta",
    "minecraft:yellow_terracotta",
];

/// The key the registrar gave a requested name, for the pieces' writers.
pub fn key_for(name: &str) -> Option<mod_api::BlockKey> {
    KEYS.get()?.get(name).copied()
}

/// The settings the server that runs now started with; the original's defaults until one starts.
/// Worldgen workers read it, and only [`read_settings`] writes it.
static SETTINGS: RwLock<Settings> = RwLock::new(Settings::DEFAULT);

/// A setting: its key, and where its value lives in [`Settings`].
enum Field {
    Int(fn(&mut Settings) -> &mut i32, f64, f64),
    Rate(fn(&mut Settings) -> &mut f64),
    Toggle(fn(&mut Settings) -> &mut bool),
}

/// Every `ConfigModule` value this code reads, with its key and, for a number, the original's
/// `defineInRange` bounds. The two heights have no range in the original; the sliders span the
/// overworld's build height.
const FIELDS: [(&str, Field); 23] = [
    ("min_y", Field::Int(|s| &mut s.min_y, -64.0, 319.0)),
    ("max_y", Field::Int(|s| &mut s.max_y, -64.0, 319.0)),
    ("lantern_rate", Field::Rate(|s| &mut s.lantern_rate)),
    ("torch_rate", Field::Rate(|s| &mut s.torch_rate)),
    ("workstation_rate", Field::Rate(|s| &mut s.workstation_rate)),
    (
        "workstation_cellar_rate",
        Field::Rate(|s| &mut s.workstation_cellar_rate),
    ),
    ("small_shaft_rate", Field::Rate(|s| &mut s.small_shaft_rate)),
    ("cobweb_rate", Field::Rate(|s| &mut s.cobweb_rate)),
    (
        "small_shaft_chest_minecart_rate",
        Field::Rate(|s| &mut s.small_shaft_chest_minecart_rate),
    ),
    (
        "small_shaft_tnt_minecart_rate",
        Field::Rate(|s| &mut s.small_shaft_tnt_minecart_rate),
    ),
    (
        "main_shaft_chest_minecart_rate",
        Field::Rate(|s| &mut s.main_shaft_chest_minecart_rate),
    ),
    (
        "main_shaft_tnt_minecart_rate",
        Field::Rate(|s| &mut s.main_shaft_tnt_minecart_rate),
    ),
    (
        "outpost_chance",
        Field::Int(|s| &mut s.zombie_room_rate, 0.0, 100.0),
    ),
    (
        "small_shaft_length",
        Field::Int(|s| &mut s.small_shaft_chain_length, 0.0, 1000.0),
    ),
    ("ore_deposits", Field::Toggle(|s| &mut s.ore_deposits)),
    (
        "cobble_chance",
        Field::Int(|s| &mut s.ore_weights[0], 0.0, 100.0),
    ),
    (
        "coal_chance",
        Field::Int(|s| &mut s.ore_weights[1], 0.0, 100.0),
    ),
    (
        "iron_chance",
        Field::Int(|s| &mut s.ore_weights[2], 0.0, 100.0),
    ),
    (
        "redstone_chance",
        Field::Int(|s| &mut s.ore_weights[3], 0.0, 100.0),
    ),
    (
        "gold_chance",
        Field::Int(|s| &mut s.ore_weights[4], 0.0, 100.0),
    ),
    (
        "lapis_chance",
        Field::Int(|s| &mut s.ore_weights[5], 0.0, 100.0),
    ),
    (
        "emerald_chance",
        Field::Int(|s| &mut s.ore_weights[6], 0.0, 100.0),
    ),
    (
        "diamond_chance",
        Field::Int(|s| &mut s.ore_weights[7], 0.0, 100.0),
    ),
];

fn declare_settings(r: &mut dyn Registrar) {
    let mut defaults = Settings::DEFAULT;
    for (key, field) in FIELDS {
        let label = format!("{}.configuration.{key}", runtime::MOD_ID);
        let spec = match field {
            Field::Int(at, min, max) => SettingSpec::range(
                key,
                label.as_str(),
                min,
                max,
                1.0,
                f64::from(*at(&mut defaults)),
            ),
            Field::Rate(at) => {
                SettingSpec::range(key, label.as_str(), 0.0, 1.0, 0.00005, *at(&mut defaults))
            }
            Field::Toggle(at) => SettingSpec::toggle(key, label.as_str(), *at(&mut defaults)),
        };
        let _ = r.register_setting(spec.tooltip(format!("{label}.tooltip")));
    }
}

/// Take the player's settings for the server about to start: NeoForge reads a `worldRestart`
/// value when the world loads, and generation reads [`settings`] from then on. A value the game
/// cannot give keeps the original's default.
pub fn read_settings(host: &dyn Host) {
    let mut read = Settings::DEFAULT;
    for (key, field) in FIELDS {
        let Some(value) = host.setting(runtime::MOD_ID, key) else {
            continue;
        };
        match field {
            Field::Toggle(at) => match value.as_str() {
                "true" => *at(&mut read) = true,
                "false" => *at(&mut read) = false,
                _ => {}
            },
            Field::Int(at, ..) => {
                if let Ok(number) = value.parse::<f64>()
                    && number.is_finite()
                {
                    *at(&mut read) = number.round() as i32;
                }
            }
            Field::Rate(at) => {
                if let Ok(number) = value.parse::<f64>()
                    && number.is_finite()
                {
                    *at(&mut read) = number;
                }
            }
        }
    }
    *SETTINGS.write().expect("the settings lock is not poisoned") = read;
}

/// The settings generation uses now.
pub fn settings() -> Settings {
    *SETTINGS.read().expect("the settings lock is not poisoned")
}

/// Register the type once, at startup: the settings, the palette's block keys, then the structure
/// type that will place them.
pub fn register(r: &mut dyn Registrar) {
    declare_settings(r);
    let mut keys = std::collections::BTreeMap::new();
    for name in REQUESTED_BLOCKS {
        keys.insert(name, r.block_key(name));
    }
    let _ = KEYS.set(keys);
    r.register_structure_type("mineshaft", MineshaftType);
}

/// How many starts the readout remembers. Worldgen keeps generating around a placement, so the
/// record is a bounded ring keyed by chunk: `/bettermineshafts last <x> <z>` answers for the chunk
/// the caller placed at, not for whatever generated last.
const LAST_STARTS_KEPT: usize = 256;

static LAST_STARTS: std::sync::Mutex<std::collections::VecDeque<([i32; 2], runtime::Assembly)>> =
    std::sync::Mutex::new(std::collections::VecDeque::new());

/// Remember the assembly a start produced (called once per start, from the worldgen worker).
pub fn record_start(chunk: [i32; 2], assembly: &runtime::Assembly) {
    let mut starts = LAST_STARTS
        .lock()
        .expect("bettermineshafts: the last-start record's lock is poisoned");
    starts.push_back((chunk, assembly.clone()));
    while starts.len() > LAST_STARTS_KEPT {
        starts.pop_front();
    }
}

/// `/bettermineshafts last [chunkX chunkZ]`: a start's chunk, position, piece counts and every
/// stair's box — the newest one, or the one at the chunk asked for.
pub fn last_lines(chunk: Option<[i32; 2]>) -> Vec<String> {
    let starts = LAST_STARTS
        .lock()
        .expect("bettermineshafts: the last-start record's lock is poisoned");
    let found = match chunk {
        Some(chunk) => starts.iter().rev().find(|(at, _)| *at == chunk),
        None => starts.back(),
    };
    let Some((chunk, assembly)) = found else {
        return match chunk {
            Some(chunk) => vec![format!(
                "{}: no start recorded at chunk {},{}",
                runtime::MOD_ID,
                chunk[0],
                chunk[1]
            )],
            None => vec![format!("{}: no start generated yet", runtime::MOD_ID)],
        };
    };
    let mut lines = vec![format!(
        "{}: last start chunk {},{} at [{}, {}, {}] -> {}",
        runtime::MOD_ID,
        chunk[0],
        chunk[1],
        assembly.start[0],
        assembly.start[1],
        assembly.start[2],
        assembly.summary(),
    )];
    for (index, piece) in assembly.pieces.iter().enumerate() {
        if piece.kind != runtime::PieceKind::Stairs {
            continue;
        }
        let min = piece.bounds.min;
        let max = piece.bounds.max;
        lines.push(format!(
            "{}: stair {index} box [{}, {}, {}]..[{}, {}, {}] facing {}",
            runtime::MOD_ID,
            min[0],
            min[1],
            min[2],
            max[0],
            max[1],
            max[2],
            piece.facing.name(),
        ));
    }
    lines
}

/// The operator commands (level 2, as vanilla's debug commands): `/bettermineshafts [status]` and `/bettermineshafts last [chunk_x chunk_z]`.
pub fn declare_commands(commands: &mut Vec<CommandRegistration>) {
    for usage in ["bettermineshafts", "bettermineshafts status"] {
        commands.push(
            Command::op(usage, 2)
                .executes(|host, _source, _invocation| Ok(send(host, runtime::status_lines()))),
        );
    }
    commands.push(
        Command::op(
            "bettermineshafts last [chunk_x:integer] [chunk_z:integer]",
            2,
        )
        .executes(|host, _source, invocation| {
            let coordinate = |name| match invocation.argument(name) {
                Some(mod_api::CommandArgumentValue::Integer(value)) => Some(*value),
                _ => None,
            };
            let chunk = coordinate("chunk_x")
                .zip(coordinate("chunk_z"))
                .map(|(x, z)| [x, z]);
            Ok(send(host, last_lines(chunk)))
        }),
    );
}

/// Send `lines` to the command's source; the command's result is their count.
fn send(host: &mut dyn ServerHost, lines: Vec<String>) -> i32 {
    let count = lines.len() as i32;
    for line in lines {
        let _ = host.commands().feedback(Text::literal(line), false);
    }
    count
}

/// A structure's palette from the NBT its definition carries: every field the config states
/// overrides [`Palette::oak_defaults`], and the rest stays oak.
pub fn palette_from(config: &Tag) -> Palette {
    let root = config.get("config").unwrap_or(config);
    let mut palette = Palette::oak_defaults();
    if let Some(rate) = number(root, "replacementRate") {
        palette.replacement_rate = rate;
    }
    if let Some(variant) = string(root, "legVariant").and_then(|name| LegVariant::from_name(&name))
    {
        palette.leg_variant = variant;
    }
    if let Some(decorations) = root.get("decorationChances") {
        palette.decorations = DecorationChances {
            vine: number(decorations, "vineChance").unwrap_or(palette.decorations.vine),
            snow: number(decorations, "snowChance").unwrap_or(palette.decorations.snow),
            cactus: number(decorations, "cactusChance").unwrap_or(palette.decorations.cactus),
            dead_bush: number(decorations, "deadBushChance")
                .unwrap_or(palette.decorations.dead_bush),
            mushroom: number(decorations, "mushroomChance").unwrap_or(palette.decorations.mushroom),
            gravel_pile: number(decorations, "gravelPileChance")
                .unwrap_or(palette.decorations.gravel_pile),
            lush: boolean(decorations, "lushDecorations").unwrap_or(palette.decorations.lush),
            dripstone: boolean(decorations, "dripstoneDecorations")
                .unwrap_or(palette.decorations.dripstone),
        };
    }
    if let Some(blocks) = root.get("blockStates") {
        let set = &mut palette.blocks;
        set.main = block(blocks, "mainBlockState").unwrap_or_else(|| set.main.clone());
        set.support = block(blocks, "supportBlockState").unwrap_or_else(|| set.support.clone());
        set.slab = block(blocks, "slabBlockState").unwrap_or_else(|| set.slab.clone());
        set.gravel = block(blocks, "gravelBlockState").unwrap_or_else(|| set.gravel.clone());
        set.stone_wall =
            block(blocks, "stoneWallBlockState").unwrap_or_else(|| set.stone_wall.clone());
        set.stone_slab =
            block(blocks, "stoneSlabBlockState").unwrap_or_else(|| set.stone_slab.clone());
        set.trapdoor = block(blocks, "trapdoorBlockState").unwrap_or_else(|| set.trapdoor.clone());
        set.small_leg =
            block(blocks, "smallLegBlockState").unwrap_or_else(|| set.small_leg.clone());
    }
    if let Some(randomizers) = root.get("blockStateRandomizers") {
        let set = &mut palette.randomizers;
        set.main = randomizer(randomizers, "mainRandomizer").unwrap_or_else(|| set.main.clone());
        set.floor = randomizer(randomizers, "floorRandomizer").unwrap_or_else(|| set.floor.clone());
        set.brick = randomizer(randomizers, "brickRandomizer").unwrap_or_else(|| set.brick.clone());
        set.leg = randomizer(randomizers, "legRandomizer").unwrap_or_else(|| set.leg.clone());
    }
    palette
}

fn string(parent: &Tag, key: &str) -> Option<String> {
    match parent.get(key) {
        Some(Tag::String(value)) => value.to_string().ok(),
        _ => None,
    }
}

fn number(parent: &Tag, key: &str) -> Option<f32> {
    match parent.get(key) {
        Some(Tag::Float(bits)) => Some(f32::from_bits(*bits)),
        Some(Tag::Double(bits)) => Some(f64::from_bits(*bits) as f32),
        Some(Tag::Int(value)) => Some(*value as f32),
        Some(Tag::Long(value)) => Some(*value as f32),
        _ => None,
    }
}

fn boolean(parent: &Tag, key: &str) -> Option<bool> {
    match parent.get(key) {
        Some(Tag::Byte(value)) => Some(*value != 0),
        _ => None,
    }
}

fn block(parent: &Tag, key: &str) -> Option<BlockRef> {
    block_ref(parent.get(key)?)
}

/// A block state's reference from the shape the carried JSON uses: `{"id": …, "properties": {…}}`,
/// with the 1.21.1 `Name`/`Properties` spelling accepted beside it.
fn block_ref(tag: &Tag) -> Option<BlockRef> {
    let name = string(tag, "id").or_else(|| string(tag, "Name"))?;
    let mut properties = Vec::new();
    if let Some(Tag::Compound(map)) = tag.get("properties").or_else(|| tag.get("Properties")) {
        for (key, value) in map {
            let Tag::String(value) = value else {
                continue;
            };
            let (Ok(key), Ok(value)) = (key.to_string(), value.to_string()) else {
                continue;
            };
            properties.push((key, value));
        }
    }
    Some(BlockRef { name, properties })
}

fn randomizer(parent: &Tag, key: &str) -> Option<Randomizer> {
    let entry = parent.get(key)?;
    let default = block_ref(entry.get("defaultBlockState")?)?;
    let mut entries = Vec::new();
    if let Some(Tag::List(list)) = entry.get("entries") {
        for weighted in list {
            let Some(state) = weighted.get("blockState").and_then(block_ref) else {
                continue;
            };
            entries.push(Weighted {
                state,
                probability: number(weighted, "probability").unwrap_or(0.0),
            });
        }
    }
    Some(Randomizer { default, entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(name: &str) -> Tag {
        Tag::compound([
            ("id", Tag::String(name.into())),
            (
                "properties",
                Tag::compound([("axis", Tag::String("y".into()))]),
            ),
        ])
    }

    fn randomizer_tag(default: &str, entry: &str, probability: f32) -> Tag {
        Tag::compound([
            ("defaultBlockState", state(default)),
            (
                "entries",
                Tag::List(vec![Tag::compound([
                    ("blockState", state(entry)),
                    // A JSON number may arrive as a double, an int or a float: all are read.
                    ("probability", Tag::Double((probability as f64).to_bits())),
                ])]),
            ),
        ])
    }

    fn desert_config() -> Tag {
        Tag::compound([
            (
                "config",
                Tag::compound([
                    ("replacementRate", Tag::Double(0.5_f64.to_bits())),
                    ("legVariant", Tag::String("inner".into())),
                    (
                        "blockStates",
                        Tag::compound([
                            ("mainBlockState", state("minecraft:sandstone")),
                            ("supportBlockState", state("minecraft:oak_fence")),
                        ]),
                    ),
                    (
                        "blockStateRandomizers",
                        Tag::compound([(
                            "mainRandomizer",
                            randomizer_tag(
                                "minecraft:sandstone",
                                "minecraft:chiseled_sandstone",
                                0.25,
                            ),
                        )]),
                    ),
                    (
                        "decorationChances",
                        Tag::compound([
                            ("cactusChance", Tag::Int(1)),
                            ("mushroomChance", Tag::Double(0.05_f64.to_bits())),
                            ("lushDecorations", Tag::Byte(1)),
                            ("vineChance", Tag::Float(0.0_f32.to_bits())),
                        ]),
                    ),
                ]),
            ),
            (
                "biomes",
                Tag::String("#bettermineshafts:has_structure/better_mineshaft_desert".into()),
            ),
        ])
    }

    #[test]
    fn the_config_overrides_the_oak_defaults_field_by_field() {
        let palette = palette_from(&desert_config());
        assert_eq!(palette.replacement_rate, 0.5);
        assert_eq!(palette.leg_variant, LegVariant::Inner);
        assert_eq!(palette.blocks.main.name, "minecraft:sandstone");
        assert_eq!(palette.blocks.main.property("axis"), Some("y"));
        assert_eq!(palette.blocks.support.name, "minecraft:oak_fence");
        // A field the config does not state keeps the oak value.
        assert_eq!(palette.blocks.gravel.name, "minecraft:gravel");
        assert_eq!(palette.decorations.cactus, 1.0);
        assert_eq!(palette.decorations.mushroom, 0.05);
        assert!(palette.decorations.lush);
        assert_eq!(palette.decorations.vine, 0.0);
        assert_eq!(palette.randomizers.main.entries.len(), 1);
        assert_eq!(
            palette.randomizers.main.entries[0].state.name,
            "minecraft:chiseled_sandstone"
        );
        assert_eq!(palette.randomizers.main.entries[0].probability, 0.25);
        assert_eq!(
            palette.randomizers.floor.default.name, "minecraft:oak_planks",
            "an unstated randomizer is the oak one"
        );
    }

    #[test]
    fn a_config_with_no_fields_is_the_oak_palette_and_the_root_is_accepted() {
        let bare = Tag::End;
        assert_eq!(palette_from(&bare), Palette::oak_defaults());
        // The config may arrive wrapped or bare; both read the same.
        let bare_config = desert_config()
            .get("config")
            .expect("the fixture nests its config")
            .clone();
        assert_eq!(palette_from(&bare_config), palette_from(&desert_config()));
    }

    #[test]
    fn the_121_and_263_spellings_of_a_block_state_both_read() {
        let legacy = Tag::compound([
            ("Name", Tag::String("minecraft:oak_log".into())),
            (
                "Properties",
                Tag::compound([("axis", Tag::String("x".into()))]),
            ),
        ]);
        let reference = block_ref(&legacy).expect("the legacy spelling reads");
        assert_eq!(reference.name, "minecraft:oak_log");
        assert_eq!(reference.property("axis"), Some("x"));
    }
}
