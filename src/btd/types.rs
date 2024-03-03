#![allow(non_camel_case_types, dead_code)]

use std::fmt::{Debug, Display};

use bytemuck::cast_slice;

use crate::{
    memory::{
        object_type, MemoryRead, Object, ObjectPointer, Pointer, ProcessMemoryView, TypeInfo,
    },
    Result,
};

macro_rules! field {
    ($offset:literal $name:ident: $field_type:ty) => {
        pub fn $name(&self) -> crate::Result<$field_type> {
            unsafe { self.field($offset) }
        }
    };
}

const TYPE_OFFSET_IN_GAME: u64 = 0x32d9b98;

// Assets_Scripts_Unity_UI_New_InGame_InGame_o
object_type!(InGame);
impl InGame {
    field!(0x00b8 unity_to_simulation: UnityToSimulation);
    field!(0x0060 input_manager: InputManager);

    pub fn get_instance(
        memory_view: &ProcessMemoryView,
        module_base: u64,
    ) -> Result<Option<InGame>> {
        let ingame_type: TypeInfo = memory_view.read(module_base + TYPE_OFFSET_IN_GAME)?;

        let ingame = ingame_type.get_statics()?.field(0x0)?;

        Ok(ingame)
    }
}

object_type!(InputManager);
impl InputManager {
    field!(0x1a8 selected: Option<TowerToSimulation>);
}

// Assets_Scripts_Unity_Bridge_UnityToSimulation_o
object_type!(UnityToSimulation);
impl UnityToSimulation {
    field!(0x0018 simulation: Simulation);
}

// Assets_Scripts_Unity_Bridge_TowerToSimulation_o
object_type!(TowerToSimulation);
impl TowerToSimulation {
    field!(0x0010 tower: Tower);
}

// Assets_Scripts_Simulation_Simulation_o
object_type!(Simulation);
impl Simulation {
    field!(0x0000 entity: Object);
    field!(0x0008 model: GameModel);
    field!(0x0050 tower_manager: TowerManager);
    field!(0x0398 map: Map);
    field!(0x0390 health: KonFuze);

    pub fn cash_manager(&self) -> Result<CashManager> {
        unsafe {
            let cash_managers: Dictionary<Object, CashManager> = self.field(0x378)?;

            Ok(cash_managers.get(0)?.1)
        }
    }
}

object_type!(Entity);
impl Entity {
    field!(0x0068 dependants: LockList<RootObject>);
}

object_type!(RootObject);
impl RootObject {}

object_type!(TowerManager);
impl TowerManager {
    // pub fn tower_history(&self) -> Result<()> {
    //     unsafe {
    //         let list: Object = self.field(0x0090)?;
    //         assert_eq!("List`1", list.get_type()?.get_name()?);

    //         let len: u32 = list.field(0x0008)?;
    //         println!("{}", len);

    //         let array: Object = list.field(0x0000)?;
    //         println!("{:?}", array.get_type()?.get_name()?);

    //         let x: Object = array.field(0x10)?;
    //         println!("{:?}", x.get_type()?.get_name()?);

    //         Ok(())
    //     }
    // }

    // pub fn towers(&self) -> Result<()> {
    //     unsafe {
    //         let list: Object = self.field(0x00c0)?;
    //         assert_eq!("List`1", list.get_type()?.get_name()?);

    //         let len: u32 = list.field(0x0008)?;
    //         println!("{}", len);

    //         let array: Object = list.field(0x0000)?;
    //         println!("{:?}", array.get_type()?.get_name()?);

    //         Ok(())
    //     }
    // }
}

// Assets_Scripts_Simulation_Track_Map_o
object_type!(Map);
impl Map {
    field!(0x0038 entity: Entity);
    field!(0x0080 path_manager: PathManager);
    field!(0x0088 spawner: Spawner);
    field!(0x0098 towers_by_area: Dictionary<Pointer, List<Tower>>);

    pub fn towers(&self) -> Result<Vec<Tower>> {
        let mut towers = vec![];

        for result in self.towers_by_area()?.iter()? {
            for result in result?.1.iter()? {
                towers.push(result?);
            }
        }

        Ok(towers)
    }
}

object_type!(PathManager);
impl PathManager {
    field!(0x0048 paths: List<Path>);
}

object_type!(Path);
impl Path {
    field!(0x0000 segments: Array<PathSegment>);
    field!(0x0008 model: PathModel);
    field!(0x0010 is_active: bool);
    field!(0x0011 is_hidden: bool);
    field!(0x0014 spawn_distance: f32);
    field!(0x0018 leak_distance: f32);
    field!(0x0120 total_path_length: f32);
    field!(0x0030 bloons: LockList<Bloon>);
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

object_type!(PathModel);
impl PathModel {}

object_type!(PathSegment);
impl PathSegment {
    field!(0x0000 bloons: List<BloonTargetProxy>);
    field!(0x0010 min: f32);
    field!(0x0014 max: f32);
    field!(0x0018 center: f32);
    field!(0x0028 leak_distance: f32);
}

object_type!(Bloon);
impl Bloon {
    field!(0x00a8 model: BloonModel);
    field!(0x0154 distance_travelled: f32);
}

object_type!(GameModel);
impl GameModel {
    field!(0x00c0 map: MapModel);
    field!(0x00c8 round_set: RoundSetModel);
    field!(0x00d0 income_set: IncomeSetModel);
    field!(0x00e0 towers: Array<TowerModel>);
    field!(0x00e8 upgrades: Array<UpgradeModel>);
    field!(0x00f0 bloons: Array<BloonModel>);
}

object_type!(MapModel);
object_type!(RoundSetModel);
object_type!(IncomeSetModel);
object_type!(BloonModel);
impl BloonModel {
    field!(0x0020 id: CSharpString);
    field!(0x0028 base_id: CSharpString);
    field!(0x00d8 children: List<BloonModel>);

    pub fn get_worth(&self) -> Result<u64> {
        let children = self
            .children()?
            .iter()?
            .map(|b| b.and_then(|v| v.get_worth()))
            .sum::<Result<u64>>()?;

        Ok(1 + children)
    }
}

object_type!(Tower);
impl Tower {
    field!(0x0090 id: CSharpString);
    field!(0x0098 worth: f32);
    field!(0x00a0 damage_dealt: u64);
    field!(0x00a8 cash_earned: u64);
    field!(0x00b0 applied_cash: f32);
    field!(0x00d0 model: TowerModel);
    field!(0x0038 entity: Option<Entity>);
}

object_type!(TowerModel);
impl TowerModel {
    field!(0x0028 base_id: CSharpString);
    field!(0x0044 tier: u32);
    field!(0x0048 tiers: Array<u32>);
    field!(0x00b8 upgrades: Array<UpgradePathModel>);
    field!(0x00c0 applied_upgrades: Array<CSharpString>);
}

object_type!(UpgradeModel);
impl UpgradeModel {
    field!(0x0000 name: CSharpString);
    field!(0x0020 cost: i32);
    field!(0x0024 xp_cost: i32);
    field!(0x0030 path: i32);
    field!(0x0034 tier: i32);
    field!(0x0038 locked: i32);
}

object_type!(UpgradePathModel);
impl UpgradePathModel {
    field!(0x0000 tower: CSharpString);
    field!(0x0008 upgrade: CSharpString);
}

object_type!(CashManager);
impl CashManager {
    field!(0x0000 cash: KonFuze);
}

// Assets_Scripts_Simulation_Track_Spawner_o
object_type!(Spawner);
impl Spawner {
    field!(0x0060 round_data: Dictionary<u32, RoundData>);
    field!(0x00d8 current_round: KonFuze_NoShuffle);
}

object_type!(RoundData);
impl RoundData {
    // field!(0x00)
}

// Assets_Scripts_Utils_KonFuze_o
object_type!(KonFuze);
impl KonFuze {
    field!(0x0018 get: f64);
}

// Assets_Scripts_Utils_KonFuze_NoShuffle_o
object_type!(KonFuze_NoShuffle);
impl KonFuze_NoShuffle {
    field!(0x0018 get: f64);
}

// Assets_Scripts_Utils_KonFuze_NoShuffle_o
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

object_type!(LockList<T>);
impl<T: MemoryRead> LockList<T> {
    field!(0x0000 list: List<T>);
}

object_type!(Dictionary<K, V>);
impl<K: MemoryRead + Debug, V: MemoryRead + Debug> Dictionary<K, V> {
    pub fn len(&self) -> Result<usize> {
        unsafe { Ok(self.field::<u32>(0x10)? as usize) }
    }

    pub fn get(&self, index: usize) -> Result<(K, V)> {
        let entries: Object = unsafe { self.field(0x8)? };
        assert_eq!("Entry[]", entries.get_type()?.get_name()?);

        // let mut buffer = vec![0; 256];
        // self.0
        //     .memory
        //     .read_exact(entries.0.address, &mut buffer)
        //     ?;
        // std::fs::File::create("output")
        //     ?
        //     .write_all(&buffer)
        //     ?;

        // println!("{} / {} / {}", index, self.len(), unsafe {
        //     entries.field::<u32>(0x0008)?
        // });
        let key = unsafe { entries.field(0x18 + 0x18 * index as u64)? };
        // println!("{:?}", key);
        let value = unsafe { entries.field(0x20 + 0x18 * index as u64)? };
        // println!("{}", index);

        Ok((key, value))
    }

    pub fn iter(&self) -> Result<impl Iterator<Item = Result<(K, V)>>> {
        let this = self.clone();
        Ok((0..this.len()?).map(move |i| this.get(i)))
    }
}
