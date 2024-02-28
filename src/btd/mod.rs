use std::collections::HashMap;

use windows::Win32::System::Threading::{
    PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
};

use crate::{
    memory::ProcessMemoryView,
    process::{Module, Process},
    Previous, Result,
};

pub mod summary;
pub mod types;
use types::{GameModel, InGame, Tower, UpgradeModel};

use self::{
    summary::{GameState, InGameState},
    types::UpgradePathModel,
};

pub fn find_pid() -> Result<u32> {
    for pid in Process::enum_process_ids() {
        if let Ok(process) = Process::from_pid(pid, PROCESS_QUERY_LIMITED_INFORMATION) {
            let file_name = process.get_image_file_name()?;

            if file_name.ends_with("BloonsTD6.exe") {
                return Ok(pid);
            }
        }
    }

    Err("bloons process not found".into())
}

pub fn find_game_module(process: &Process) -> Result<Module> {
    for module in process.get_modules()? {
        let module_name = module.get_base_name()?;

        if module_name == "GameAssembly.dll" {
            return Ok(module);
        }
    }

    Err("module not found".into())
}

pub fn get_cash(ingame: &InGame) -> Result<u64> {
    Ok(ingame
        .unity_to_simulation()?
        .simulation()?
        .cash_manager()?
        .cash()?
        .get()? as u64)
}

pub fn get_available_upgrades(
    model_cache: &ModelCache,
    tower: &Tower,
) -> Result<Vec<(Tower, UpgradeModel, u64)>> {
    let mut options = vec![];
    for upgrade in tower.model()?.upgrades()?.iter()? {
        let upgrade = upgrade?;
        let upgrade = model_cache.get_upgrade(&upgrade)?;

        options.push((tower.clone(), upgrade.clone(), upgrade.cost()? as u64));
    }

    options.sort_by_key(|v| v.2);

    Ok(options)
}

pub fn get_all_available_upgrades(
    model_cache: &ModelCache,
    ingame: &InGame,
) -> Result<Vec<(Tower, UpgradeModel, u64)>> {
    let simulation = ingame.unity_to_simulation()?.simulation()?;

    let mut options = vec![];
    for tower in simulation.map()?.towers()? {
        for upgrade in tower.model()?.upgrades()?.iter()? {
            let upgrade = upgrade?;
            let upgrade = model_cache.get_upgrade(&upgrade)?;

            options.push((tower.clone(), upgrade.clone(), upgrade.cost()? as u64));
        }
    }

    options.sort_by_key(|v| v.2);

    Ok(options)
}

pub struct ModelCache {
    upgrades: HashMap<String, UpgradeModel>,
}

impl ModelCache {
    pub fn load(model: &GameModel) -> Result<ModelCache> {
        let mut upgrades = HashMap::new();

        for upgrade_model in model.upgrades()?.iter()? {
            let upgrade_model = upgrade_model?;

            upgrades.insert(upgrade_model.name()?.to_string(), upgrade_model);
        }

        Ok(Self { upgrades })
    }

    pub fn get_upgrade(&self, id: &UpgradePathModel) -> Result<&UpgradeModel> {
        Ok(self
            .upgrades
            .get(&id.upgrade()?.to_string())
            .expect(&format!("upgrade not found: {}", id.upgrade()?)))
    }
}

pub struct BloonsGame {
    ingame_addr: Previous<u64>,
    model_cache: Option<ModelCache>,

    memory: ProcessMemoryView,
    module_offset: u64,
}

impl BloonsGame {
    pub fn new(memory: ProcessMemoryView, module_offset: u64) -> Self {
        Self {
            ingame_addr: Default::default(),
            model_cache: Default::default(),
            memory,
            module_offset,
        }
    }

    pub fn find_game() -> Result<Self> {
        let pid = find_pid()?;

        let process = Process::from_pid(pid, PROCESS_QUERY_INFORMATION | PROCESS_VM_READ)?;

        let memory = ProcessMemoryView::new(process);
        let module = find_game_module(&process)?;

        Ok(Self::new(memory, module.get_bounds()?.0))
    }

    pub fn get_state(&mut self) -> GameState {
        let state = match self.try_get_state() {
            Ok(s) => s,
            Err(_) => GameState::None,
        };

        state
    }

    pub fn try_get_state(&mut self) -> Result<GameState> {
        match InGame::get_instance(&self.memory, self.module_offset)? {
            None => Ok(GameState::None),
            Some(ingame) => {
                if self.ingame_addr.set(ingame.0.address) {
                    self.model_cache = None;
                }

                let model_cache = match self.model_cache.as_ref() {
                    Some(m) => m,
                    None => {
                        self.model_cache = Some(ModelCache::load(
                            &ingame.unity_to_simulation()?.simulation()?.model()?,
                        )?);
                        self.model_cache.as_ref().unwrap()
                    }
                };

                let state = InGameState::load(model_cache, &ingame)?;

                Ok(GameState::InGame(state))
            }
        }
    }
}
