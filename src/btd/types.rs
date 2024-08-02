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

// everything below this is 1:1 from decompiled source

// Namespace: Assets.Scripts.Unity.UI_New.InGame
object_type!(InGame);
impl InGame {
    // private InputManager inputManager; // 0x70
    field!(0x0060 input_manager: InputManager);

    // private UnityToSimulation bridge; // 0xC8
    field!(0x00b8 unity_to_simulation: UnityToSimulation);

    // private bool stoppedClockForMenuOpen; // 0x170
    field!(0x0160 stopped_clock_for_menu_open: bool);
}

// Namespace: Assets.Scripts.Unity.UI_New.InGame
object_type!(InputManager);
impl InputManager {
    // private Selectable selected; // 0x1B0
    field!(0x1a0 selected: Option<Object>);
}

// Namespace: Assets.Scripts.Unity.Bridge
object_type!(UnityToSimulation);
impl UnityToSimulation {
    // protected Simulation simulation; // 0x28
    field!(0x0018 simulation: Simulation);
}

// Namespace: Assets.Scripts.Unity.Bridge
object_type!(TowerToSimulation);
impl TowerToSimulation {
    // private Tower tower; // 0x20
    field!(0x0010 tower: Tower);
}

// Namespace: Assets.Scripts.Simulation
object_type!(Simulation);
impl Simulation {
    // public Entity entity; // 0x10
    field!(0x0000 entity: Object);
    // public GameModel model; // 0x20
    field!(0x0010 model: GameModel);
    // public Time time; // 0x30
    field!(0x0020 time: Time);
    // public Time roundTime; // 0x38
    field!(0x0028 round_time: Time);
    // public TowerManager towerManager; // 0x68
    field!(0x0058 tower_manager: TowerManager);
    // private readonly Dictionary<int, Simulation.CashManager> cashManagers; // 0x3E8
    field!(0x03d8 cash_managers: Dictionary<Object, CashManager>);
    // private KonFuze health; // 0x408
    field!(0x03f8 health: KonFuze);
    // private Map map; // 0x410
    field!(0x0400 map: Map);
}

// Namespace: Assets.Scripts.Simulation
object_type!(Time);
impl Time {
    // public int elapsed; // 0x10
    field!(0x0000 elapsed: i32);
}

// Namespace: Assets.Scripts.Simulation.Objects
object_type!(Entity);
impl Entity {
    // private LockList<RootObject> dependants; // 0x70
    field!(0x0060 dependants: LockList<RootObject>);
}

// Namespace: Assets.Scripts.Simulation.Objects
object_type!(RootObject);

// Namespace: Assets.Scripts.Simulation.Towers
object_type!(TowerManager);

//  Assets.Scripts.Simulation.Track
object_type!(Map);
impl Map {
    // Namespace: Assets.Scripts.Simulation.Objects.RootBehavior
    // public Entity entity; // 0x48
    field!(0x0038 entity: Entity);
    // public MapModel mapModel; // 0x58
    field!(0x0048 model: MapModel);
    // public PathManager pathManager; // 0x90
    field!(0x0080 path_manager: PathManager);
    // public Spawner spawner; // 0x98
    field!(0x0088 spawner: Spawner);
    // public Dictionary<ObjectId, List<Tower>> areaTowers; // 0xA8
    field!(0x0098 towers_by_area: Dictionary<Pointer, List<Tower>>);
}

// Namespace: Assets.Scripts.Simulation.Track
object_type!(PathManager);
impl PathManager {
    // public List<Path> paths; // 0x58
    field!(0x0048 paths: List<Path>);
}

// Namespace: Assets.Scripts.Simulation.Track
object_type!(Path);
impl Path {
    // public PathSegment[] segments; // 0x10
    field!(0x0000 segments: Array<PathSegment>);
    // public PathModel def; // 0x18
    field!(0x0008 model: PathModel);
    // public bool isActive; // 0x20
    field!(0x0010 is_active: bool);
    // public bool isHidden; // 0x21
    field!(0x0011 is_hidden: bool);
    // public float spawnDist; // 0x24
    field!(0x0014 spawn_distance: f32);
    // public float leakDist; // 0x28
    field!(0x0018 leak_distance: f32);
    // public readonly LockList<Bloon> bloonsList; // 0x40
    field!(0x0030 bloons: LockList<Bloon>);
    // private float totalPathLength; // 0x130
    field!(0x0120 total_path_length: f32);
}

// Namespace: Assets.Scripts.Simulation.Track
object_type!(PathModel);
impl PathModel {}

// Namespace: Assets.Scripts.Simulation.Track
object_type!(PathSegment);
impl PathSegment {
    // public List<BloonTargetProxy> bloons; // 0x10
    field!(0x0000 bloons: List<BloonTargetProxy>);
    // public readonly float min; // 0x20
    field!(0x0010 min: f32);
    // public readonly float max; // 0x24
    field!(0x0014 max: f32);
    // public readonly float centre; // 0x28
    field!(0x0018 center: f32);
    // public readonly float distanceUntilLeak; // 0x38
    field!(0x0028 leak_distance: f32);
}

// Namespace: Assets.Scripts.Simulation.Bloons
object_type!(Bloon);
impl Bloon {
    // public BloonModel bloonModel; // 0xB8
    field!(0x00a8 model: BloonModel);
    // private float distanceTraveled; // 0x168
    field!(0x0158 distance_travelled: f32);
}

// Namespace: Assets.Scripts.Models
object_type!(GameModel);
impl GameModel {
    // public string difficultyId; // 0x58
    field!(0x0048 difficulty_id: CSharpString);
    // public string gameType; // 0x80
    field!(0x0070 game_type: CSharpString);
    // public string gameMode; // 0x88
    field!(0x0078 game_mode: CSharpString);
    // public int randomSeed; // 0x94
    field!(0x0084 random_seed: i32);
    // public bool reverseMode; // 0x98
    field!(0x0088 reverse_mode: bool);
    // public MapModel map; // 0xE0
    field!(0x00d0 map: MapModel);
    // private RoundSetModel <roundSet>k__BackingField; // 0xE8
    field!(0x00d8 round_set: RoundSetModel);
    // private IncomeSetModel <incomeSet>k__BackingField; // 0xF0
    field!(0x00d0 income_set: IncomeSetModel);
    // public TowerModel[] towers; // 0x100
    field!(0x00f0 towers: Array<TowerModel>);
    // public UpgradeModel[] upgrades; // 0x108
    field!(0x00f8 upgrades: Array<UpgradeModel>);
    // public BloonModel[] bloons; // 0x110
    field!(0x0100 bloons: Array<BloonModel>);
}

// Namespace: Assets.Scripts.Models.Map
object_type!(MapModel);
impl MapModel {
    // public readonly int mapDifficulty; // 0x78
    field!(0x0068 map_difficulty: i32);
    // public string mapName; // 0x80
    field!(0x0070 map_name: CSharpString);
}

// Namespace: Assets.Scripts.Models.Rounds
object_type!(RoundSetModel);
impl RoundSetModel {
    // public readonly RoundModel[] rounds; // 0x30
    field!(0x0020 rounds: Array<RoundModel>);
}

// Namespace: Assets.Scripts.Models.Rounds
object_type!(RoundModel);
impl RoundModel {
    // public BloonGroupModel[] groups; // 0x30
    field!(0x0020 groups: Array<BloonGroupModel>);
    // private BloonEmissionModel[] emissions_; // 0x38
    field!(0x0028 emissions: Option<Array<BloonEmissionModel>>);
}

// Namespace: Assets.Scripts.Models.Rounds
object_type!(BloonGroupModel);
impl BloonGroupModel {
    // public string bloon; // 0x30
    field!(0x0020 bloon: CSharpString);
    // public float start; // 0x38
    field!(0x0028 start: f32);
    // public float end; // 0x3C
    field!(0x002c end: f32);
    // public int count; // 0x40
    field!(0x0030 count: i32);
}

// Namespace: Assets.Scripts.Models.Rounds
object_type!(BloonEmissionModel);
impl BloonEmissionModel {
    // public string bloon; // 0x30
    field!(0x0020 bloon: CSharpString);
    // public float time; // 0x38
    field!(0x0028 time: f32);
    // public int emissionIndex; // 0x3C
    field!(0x002c emission_index: i32);
    // public bool isCustomBossEmission; // 0x40
    field!(0x0030 is_custom_boss_emission: bool);
    // public TowerSet towerSetImmunity; // 0x44
    field!(0x0034 tower_set_immunity: u32);
}

// Namespace: Assets.Scripts.Models.Rounds
object_type!(IncomeSetModel);

// Namespace: Assets.Scripts.Models.Bloons
object_type!(BloonModel);
impl BloonModel {
    // public string id; // 0x30
    field!(0x0020 id: CSharpString);
    // public string baseId; // 0x38
    field!(0x0028 base_id: CSharpString);
    // public int maxHealth; // 0xB4
    field!(0x00a4 max_health: i32);
    // public float leakDamage; // 0xC0
    field!(0x00b0 leak_damage: f32);
    // public int layerNumber; // 0xC4
    field!(0x00b4 layer_number: i32);
    // private readonly List<BloonModel> childBloonModels; // 0xE8
    field!(0x00d8 children: List<BloonModel>);
}

// Namespace: Assets.Scripts.Simulation.Towers
object_type!(Tower);
impl Tower {
    // Namespace: Assets.Scripts.Simulation.Objects.RootBehavior
    // public Entity entity; // 0x48
    field!(0x0038 entity: Option<Entity>);
    // public string uniqueId; // 0xA0
    field!(0x0090 id: CSharpString);
    // public float worth; // 0xA8
    field!(0x0098 worth: f32);
    // public long damageDealt; // 0xB0
    field!(0x00a0 damage_dealt: u64);
    // public long cashEarned; // 0xB8
    field!(0x00a8 cash_earned: u64);
    // public float appliedCash; // 0xC0
    field!(0x00b0 applied_cash: f32);
    // public ObjectId parentTowerId; // 0xD4
    field!(0x00c4 parent_tower_id: ObjectId);
    // public TowerModel towerModel; // 0xE0
    field!(0x00d0 model: TowerModel);
}

object_type!(TowerModel);
impl TowerModel {
    // public string baseId; // 0x38
    field!(0x0028 base_id: CSharpString);
    // public int tier; // 0x54
    field!(0x0044 tier: u32);
    // public int[] tiers; // 0x58
    field!(0x0048 tiers: Array<u32>);
    // public TowerSet towerSet; // 0x60
    field!(0x0050 tower_set: u32);
    // public UpgradePathModel[] upgrades; // 0xC8
    field!(0x00b8 upgrades: Array<UpgradePathModel>);
    // public string[] appliedUpgrades; // 0xD0
    field!(0x00c0 applied_upgrades: Array<CSharpString>);
    // public bool isBakable; // 0xE9
    field!(0x00d9 is_bakable: bool);
}

// Namespace: Assets.Scripts.Models.Towers.Upgrades
object_type!(UpgradeModel);
impl UpgradeModel {
    // Assets.Scripts.Models.Model
    // private string _name; // 0x10
    field!(0x0000 name: CSharpString);
    // public int cost; // 0x30
    field!(0x0020 cost: i32);
    // public int xpCost; // 0x34
    field!(0x0024 xp_cost: i32);
    // public int path; // 0x40
    field!(0x0030 path: i32);
    // public int tier; // 0x44
    field!(0x0034 tier: i32);
    // public int locked; // 0x48
    field!(0x0038 locked: i32);
}

// Namespace: Assets.Scripts.Models.Towers.Upgrades
object_type!(UpgradePathModel);
impl UpgradePathModel {
    // public readonly string tower; // 0x10
    field!(0x0000 tower: CSharpString);
    // public readonly string upgrade; // 0x18
    field!(0x0008 upgrade: CSharpString);
}

object_type!(CashManager);
impl CashManager {
    // public readonly KonFuze cash; // 0x10
    field!(0x0000 cash: KonFuze);
}

// Namespace: Assets.Scripts.Simulation.Track
object_type!(Spawner);
impl Spawner {
    // public Dictionary<int, RoundData> roundData; // 0x70
    field!(0x0060 round_data: Dictionary<u32, RoundData>);
    // private KonFuze_NoShuffle currentRound; // 0xF8
    field!(0x00e8 current_round: KonFuze_NoShuffle);
}

// Namespace: Assets.Scripts.Simulation.Track
object_type!(RoundData);

// Namespace: Assets.Scripts.Utils
object_type!(KonFuze);
impl KonFuze {
    // private double honey; // 0x28
    field!(0x0018 get: f64);
}

// Namespace: Assets.Scripts.Utils
object_type!(KonFuze_NoShuffle);
impl KonFuze_NoShuffle {
    // Assets.Scripts.Utils.KonFuze
    // private double honey; // 0x28
    field!(0x0018 get: f64);
}

// Namespace: Assets.Scripts.Utils
object_type!(LockList<T>);
impl<T: MemoryRead> LockList<T> {
    // private readonly List<T> list; // 0x0
    field!(0x0000 list: List<T>);
}
