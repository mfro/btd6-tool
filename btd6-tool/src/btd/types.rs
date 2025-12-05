#![allow(non_camel_case_types, dead_code)]

use std::fmt::{Debug, Display};

use bytemuck::cast_slice;

use crate::{
    memory::{object_type, MemoryRead, Object, ObjectPointer, Pointer, ProcessMemoryView},
    Result,
};

macro_rules! field {
    ($offset:literal $name:ident: $field_type:ty) => {
        pub fn $name(&self) -> crate::Result<$field_type> {
            unsafe { self.field($offset) }
        }
    };
}
pub(crate) use field;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectId(u32);

impl ObjectId {
    pub const INVALID: ObjectId = ObjectId(4294967295);
}

impl MemoryRead for ObjectId {
    const SIZE: usize = u32::SIZE;

    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        Ok(Self(view.read(address)?))
    }
}

#[derive(Debug, Clone)]
pub struct BloonTargetProxy {
    pub bloon: Bloon,
    pub segment: PathSegment,
}

impl MemoryRead for BloonTargetProxy {
    const SIZE: usize = Bloon::SIZE + PathSegment::SIZE;

    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let bloon = view.read(address)?;
        let segment = view.read(address + Bloon::SIZE as u64)?;

        Ok(Self { bloon, segment })
    }
}

object_type!(CSharpString ; "String");
impl CSharpString {
    pub fn len(&self) -> Result<usize> {
        Ok(unsafe { self.field::<u32>(0x0000)? as usize })
    }
}

impl Display for CSharpString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.len().unwrap();

        let mut data = vec![0; 2 * len];
        self.0
            .memory
            .read_exact(self.0.address + 0x0014, &mut data)
            .unwrap();

        let str = String::from_utf16(cast_slice(&data)).unwrap();

        Display::fmt(&str, f)
    }
}

object_type!(Array<T>);
impl<T: MemoryRead> Array<T> {
    pub fn len(&self) -> Result<usize> {
        unsafe { Ok(self.field::<u32>(0x0008)? as usize) }
    }

    pub fn get(&self, index: usize) -> Result<T> {
        unsafe { Ok(self.field((0x0010 + T::SIZE * index) as u64)?) }
    }

    pub fn iter<'a>(&'a self) -> Result<impl Iterator<Item = Result<T>> + 'a> {
        Ok((0..self.len()?).map(|i| self.get(i)))
    }
}

object_type!(List<T>);
impl<T: MemoryRead> List<T> {
    pub fn len(&self) -> Result<usize> {
        unsafe { Ok(self.field::<u32>(0x0008)? as usize) }
    }

    pub fn get(&self, index: usize) -> Result<T> {
        let array: Array<T> = unsafe { self.field(0x0000)? };

        Ok(array.get(index)?)
    }

    pub fn iter(&self) -> Result<impl Iterator<Item = Result<T>>> {
        let this = self.clone();
        Ok((0..this.len()?).map(move |i| this.get(i)))
    }
}

object_type!(Dictionary<K, V>);
impl<K: MemoryRead + Debug, V: MemoryRead + Debug> Dictionary<K, V> {
    pub fn len(&self) -> Result<usize> {
        unsafe { Ok(self.field::<u32>(0x10)? as usize) }
    }

    pub fn get(&self, index: usize) -> Result<(K, V)> {
        let entries: Object = unsafe { self.field(0x8)? };
        assert_eq!("Entry[]", entries.get_type()?.get_name()?);

        let key = unsafe { entries.field(0x18 + 0x18 * index as u64)? };
        let value = unsafe { entries.field(0x20 + 0x18 * index as u64)? };

        Ok((key, value))
    }

    pub fn iter(&self) -> Result<impl Iterator<Item = Result<(K, V)>>> {
        let this = self.clone();
        Ok((0..this.len()?).map(move |i| this.get(i)))
    }
}

object_type!(CashManager);
impl CashManager {
    // public readonly KonFuze cash; // 0x10
    field!(0x0000 cash: KonFuze);
}

// Namespace: Assets.Scripts.Utils
object_type!(LockList<T>);
impl<T: MemoryRead> LockList<T> {
    // private readonly List<T> list; // 0x0
    field!(0x0000 list: List<T>);
}

// Namespace: Assets.Scripts.Models.TowerSets
pub struct TowerSet;
impl TowerSet {
    pub const NONE: u32 = 0;
    pub const PRIMARY: u32 = 1;
    pub const MILITARY: u32 = 2;
    pub const MAGIC: u32 = 4;
    pub const SUPPORT: u32 = 8;
    pub const HERO: u32 = 16;
    pub const PARAGON: u32 = 32;
    pub const ITEMS: u32 = 64;
}

#[btd6_tool_bindgen::class(rename = "PlayerContext.Context", namespace = "")]
pub struct PlayerContext_Context {
    #[rename = "inputManager"]
    input_manager: InputManager,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Unity.UI_New.InGame")]
pub struct InGame {
    #[rename = "playerContexts"]
    player_contexts: List<PlayerContext_Context>,
    #[rename = "bridge"]
    unity_to_simulation: UnityToSimulation,
    #[rename = "stoppedClockForMenuOpen"]
    stopped_clock_for_menu_open: bool,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Unity.UI_New.InGame")]
pub struct InputManager {
    #[rename = "selected"]
    selected: Option<Object>,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Unity.Bridge")]
pub struct UnityToSimulation {
    #[rename = "simulation"]
    simulation: Simulation,
    #[rename = "ttss"]
    towers: List<TowerToSimulation>,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Unity.Bridge")]
pub struct TowerToSimulation {
    #[rename = "tower"]
    tower: Tower,
    #[rename = "result"]
    abilities: List<AbilityToSimulation>,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Unity.Bridge")]
pub struct AbilityToSimulation {
    #[rename = "ability"]
    ability: Ability,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Towers.Behaviors.Abilities")]
pub struct Ability {
    #[rename = "abilityModel"]
    model: AbilityModel,
    #[rename = "cooldownTimeRemaining"]
    cooldown_remaining: f32,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models.Towers.Behaviors.Abilities")]
pub struct AbilityModel {
    #[rename = "displayName"]
    name: CSharpString,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation")]
pub struct Simulation {
    #[rename = "entity"]
    entity: Object,
    #[rename = "model"]
    model: GameModel,
    #[rename = "time"]
    time: SimulationTime,
    #[rename = "roundTime"]
    round_time: SimulationTime,
    #[rename = "towerManager"]
    tower_manager: TowerManager,
    #[rename = "cashManagers"]
    cash_managers: Dictionary<Object, CashManager>,
    #[rename = "health"]
    health: KonFuze,
    #[rename = "map"]
    map: Map,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation")]
pub struct SimulationTime {
    #[rename = "elapsed"]
    elapsed: i32,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Objects")]
pub struct Entity {
    #[rename = "dependants"]
    dependants: LockList<RootObject>,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Objects")]
pub struct RootObject {
    #[rename = "<Id>k__BackingField"]
    object_id: CSharpString,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Objects", base = RootObject)]
pub struct RootBehavior {
    #[rename = "entity"]
    entity: Option<Entity>,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Towers")]
pub struct TowerManager {}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Track", base = RootBehavior)]
pub struct Map {
    #[rename = "mapModel"]
    model: MapModel,
    #[rename = "pathManager"]
    path_manager: PathManager,
    #[rename = "spawner"]
    spawner: Spawner,
    #[rename = "areaTowers"]
    towers_by_area: Dictionary<Pointer, List<Tower>>,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Track")]
pub struct PathManager {
    #[rename = "paths"]
    paths: List<Path>,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Track")]
pub struct Path {
    #[rename = "segments"]
    segments: Array<PathSegment>,
    #[rename = "def"]
    model: PathModel,
    #[rename = "isActive"]
    is_active: bool,
    #[rename = "isHidden"]
    is_hidden: bool,
    #[rename = "spawnDist"]
    spawn_distance: f32,
    #[rename = "leakDist"]
    leak_distance: f32,
    #[rename = "bloonsList"]
    bloons: LockList<Bloon>,
    #[rename = "totalPathLength"]
    total_path_length: f32,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Track")]
pub struct PathSegment {
    #[rename = "bloons"]
    bloons: List<BloonTargetProxy>,
    #[rename = "min"]
    min: f32,
    #[rename = "max"]
    max: f32,
    #[rename = "centre"]
    center: f32,
    #[rename = "distanceUntilLeak"]
    leak_distance: f32,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Track")]
pub struct Spawner {
    #[rename = "roundData"]
    round_data: Dictionary<u32, RoundData>,
    #[rename = "currentRound"]
    current_round: KonFuze_NoShuffle,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Track")]
pub struct RoundData {}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models.Map")]
pub struct PathModel {}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Behaviors", base = RootBehavior)]
pub struct DisplayBehavior {
    #[rename = "DisplayCategory"]
    display_category: u32,
    #[rename = "processing"]
    processing: bool,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Towers")]
pub struct Tower {
    #[rename = "uniqueId"]
    id: CSharpString,
    #[rename = "worth"]
    worth: f32,
    #[rename = "damageDealt"]
    damage_dealt: u64,
    #[rename = "cashEarned"]
    cash_earned: u64,
    #[rename = "appliedCash"]
    applied_cash: f32,
    #[rename = "towerModel"]
    model: TowerModel,
    #[rename = "areaPlacedOn"]
    area_placed_on: ObjectId,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Simulation.Bloons")]
pub struct Bloon {
    #[rename = "bloonModel"]
    model: BloonModel,
    #[rename = "distanceTraveled"]
    distance_travelled: f32,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models")]
pub struct Model {
    #[rename = "_name"]
    name: CSharpString,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models")]
pub struct GameModel {
    #[rename = "difficultyId"]
    difficulty_id: CSharpString,
    #[rename = "gameType"]
    game_type: CSharpString,
    #[rename = "gameMode"]
    game_mode: CSharpString,
    #[rename = "randomSeed"]
    random_seed: i32,
    #[rename = "reverseMode"]
    reverse_mode: bool,
    #[rename = "map"]
    map: MapModel,
    #[rename = "<roundSet>k__BackingField"]
    round_set: RoundSetModel,
    #[rename = "<incomeSet>k__BackingField"]
    income_set: IncomeSetModel,
    #[rename = "towers"]
    towers: Array<TowerModel>,
    #[rename = "upgrades"]
    upgrades: Array<UpgradeModel>,
    #[rename = "bloons"]
    bloons: Array<BloonModel>,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models.Map")]
pub struct MapModel {
    #[rename = "mapDifficulty"]
    map_difficulty: i32,
    #[rename = "mapName"]
    map_name: CSharpString,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models.Rounds")]
pub struct RoundModel {
    #[rename = "groups"]
    groups: Array<BloonGroupModel>,
    #[rename = "emissions_"]
    emissions: Option<Array<BloonEmissionModel>>,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models.Rounds")]
pub struct RoundSetModel {
    #[rename = "rounds"]
    rounds: Array<RoundModel>,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models.Rounds")]
pub struct BloonGroupModel {
    #[rename = "bloon"]
    bloon: CSharpString,
    #[rename = "start"]
    start: f32,
    #[rename = "end"]
    end: f32,
    #[rename = "count"]
    count: i32,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models.Rounds")]
pub struct BloonEmissionModel {
    #[rename = "bloon"]
    bloon: CSharpString,
    #[rename = "time"]
    time: f32,
    #[rename = "emissionIndex"]
    emission_index: i32,
    #[rename = "isCustomBossEmission"]
    is_custom_boss_emission: bool,
    #[rename = "towerSetImmunity"]
    tower_set_immunity: u32,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models.Rounds")]
pub struct IncomeSetModel {}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models.Entities")]
pub struct EntityModel {
    #[rename = "baseId"]
    base_id: CSharpString,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models.Bloons", base = Model)]
pub struct BloonModel {
    #[rename = "id"]
    id: CSharpString,
    #[rename = "baseId"]
    base_id: CSharpString,
    #[rename = "maxHealth"]
    max_health: i32,
    #[rename = "leakDamage"]
    leak_damage: f32,
    #[rename = "layerNumber"]
    layer_number: i32,
    #[rename = "childBloonModels"]
    children: List<BloonModel>,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models.Towers", base = EntityModel)]
pub struct TowerModel {
    #[rename = "tier"]
    tier: u32,
    #[rename = "tiers"]
    tiers: Array<u32>,
    #[rename = "towerSet"]
    tower_set: u32,
    #[rename = "upgrades"]
    upgrades: Array<UpgradePathModel>,
    #[rename = "appliedUpgrades"]
    applied_upgrades: Array<CSharpString>,
    #[rename = "isBakable"]
    is_bakable: bool,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models.Towers.Upgrades", base = Model)]
pub struct UpgradeModel {
    #[rename = "cost"]
    cost: i32,
    #[rename = "xpCost"]
    xp_cost: i32,
    #[rename = "path"]
    path: i32,
    #[rename = "tier"]
    tier: i32,
    #[rename = "locked"]
    locked: i32,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Models.Towers.Upgrades")]
pub struct UpgradePathModel {
    #[rename = "tower"]
    tower: CSharpString,
    #[rename = "upgrade"]
    upgrade: CSharpString,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Utils")]
pub struct KonFuze {
    #[rename = "honey"]
    get: f64,
}

#[btd6_tool_bindgen::class(namespace = "Assets.Scripts.Utils", base = KonFuze)]
pub struct KonFuze_NoShuffle {}
