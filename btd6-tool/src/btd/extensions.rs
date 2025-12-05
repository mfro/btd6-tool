use anyhow::bail;

use crate::{
    memory::{ProcessMemoryView, TypeInfo},
    Result,
};

use super::types::*;

// see extract.sh
const TYPE_OFFSET_IN_GAME: u64 = 76052240;

impl InGame {
    pub fn get_instance(
        memory_view: &ProcessMemoryView,
        module_base: u64,
    ) -> Result<Option<InGame>> {
        let ingame_type: TypeInfo = memory_view.read(module_base + TYPE_OFFSET_IN_GAME)?;

        let ingame = ingame_type.get_statics()?.field(0x0)?;

        Ok(ingame)
    }
}

impl Simulation {
    pub fn cash_manager(&self) -> Result<CashManager> {
        let cash_managers = self.cash_managers()?;
        if cash_managers.len()? != 1 {
            bail!("cash manager count")
        } else {
            Ok(cash_managers.get(0)?.1)
        }
    }
}

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

impl Map {
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

impl GameModel {
    pub fn get_identifier(&self) -> Result<String> {
        let mode = self.game_mode()?;
        let map_name = self.map()?.map_name()?;

        assert_eq!("Standard", self.game_type()?.to_string());

        let identifier = format!("{} - {}", map_name, mode);

        Ok(identifier)
    }
}

impl BloonModel {
    pub fn count_rbe(&self) -> Result<u64> {
        let children = self
            .children()?
            .iter()?
            .map(|b| b.and_then(|v| v.count_rbe()))
            .sum::<Result<u64>>()?;

        Ok(self.max_health()? as u64 + children)
    }

    pub fn count_worth(&self) -> Result<u64> {
        let children = self
            .children()?
            .iter()?
            .map(|b| b.and_then(|v| v.count_worth()))
            .sum::<Result<u64>>()?;

        Ok(1 + children)
    }
}

impl Tower {
    pub fn is_real(&self) -> Result<bool> {
        Ok(self.area_placed_on()? != ObjectId::INVALID)
    }
}
