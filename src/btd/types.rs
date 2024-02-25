#![allow(non_camel_case_types)]

use crate::{
    memory::{object_type, Object, ObjectPointer, ProcessMemoryView, TypeInfo},
    Result,
};

macro_rules! field {
    (@$offset:literal $name:ident: $field_type:ty) => {
        pub fn $name(&self) -> $field_type {
            unsafe { self.field($offset).unwrap() }
        }
    };
}

const TYPE_OFFSET_IN_GAME: u64 = 0x32d9b98;

// Assets_Scripts_Unity_UI_New_InGame_InGame_o
object_type!(InGame);
impl InGame {
    field!(@0x00b8 unity_to_simulation: UnityToSimulation);

    pub fn get_instance(memory_view: &ProcessMemoryView, module_base: u64) -> Option<InGame> {
        let ingame_type: TypeInfo = memory_view.read(module_base + TYPE_OFFSET_IN_GAME).unwrap();

        let ingame = ingame_type.get_statics().field(0x0).unwrap();

        ingame
    }
}

// Assets_Scripts_Unity_Bridge_UnityToSimulation_o
object_type!(UnityToSimulation);
impl UnityToSimulation {
    field!(@0x0018 simulation: Simulation);
}

// Assets_Scripts_Simulation_Simulation_o
object_type!(Simulation);
impl Simulation {
    field!(@0x0050 tower_manager: TowerManager);
    field!(@0x0398 map: Map);
    field!(@0x0390 health: KonFuze);

    pub fn cash_manager(&self) -> CashManager {
        unsafe {
            let cash_managers: Object = self.field(0x378).unwrap();
            assert_eq!("Dictionary`2", cash_managers.get_type().get_name());

            // System_Collections_Generic_Dictionary_Entry_TKey__TValue__array
            let cash_manager_entries: Object = cash_managers.field(0x8).unwrap();
            assert_eq!("Entry[]", cash_manager_entries.get_type().get_name());

            // Assets_Scripts_Simulation_Simulation_CashManager_o
            let cash_manager = cash_manager_entries.field(0x20).unwrap();

            cash_manager
        }
    }
}

object_type!(TowerManager);
impl TowerManager {
    pub fn tower_history(&self) {
        unsafe {
            let list: Object = self.field(0x0090).unwrap();
            assert_eq!("List`1", list.get_type().get_name());

            let len: u32 = list.field(0x0008).unwrap();
            println!("{}", len);

            let array: Object = list.field(0x0000).unwrap();
            println!("{:?}", array.get_type().get_name());

            let x: Object = array.field(0x10).unwrap();
            println!("{:?}", x.get_type().get_name());
        }
    }

    pub fn towers(&self) {
        unsafe {
            let list: Object = self.field(0x00c0).unwrap();
            assert_eq!("List`1", list.get_type().get_name());

            let len: u32 = list.field(0x0008).unwrap();
            println!("{}", len);

            let array: Object = list.field(0x0000).unwrap();
            println!("{:?}", array.get_type().get_name());

            // let x: Object = array.field(0x10).unwrap();
            // println!("{:?}", x.get_type().get_name());
            // println!("{:?}", x.get_type().get_name());
        }
    }
}

// Assets_Scripts_Simulation_Track_Map_o
object_type!(Map);
impl Map {
    field!(@0x0088 spawner: Spawner);
}

object_type!(CashManager);
impl CashManager {
    field!(@0x0000 cash: KonFuze);
}

// Assets_Scripts_Simulation_Track_Spawner_o
object_type!(Spawner);
impl Spawner {
    field!(@0x00d8 round: KonFuze_NoShuffle);
}

// Assets_Scripts_Utils_KonFuze_o
object_type!(KonFuze);
impl KonFuze {
    field!(@0x0018 get: f64);
}

// Assets_Scripts_Utils_KonFuze_NoShuffle_o
object_type!(KonFuze_NoShuffle);
impl KonFuze_NoShuffle {
    field!(@0x0018 get: f64);
}
