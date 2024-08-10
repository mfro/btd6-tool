#![allow(dead_code)]

use std::collections::HashMap;

use anyhow::bail;
use windows::Win32::System::Threading::{
    PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
};

use crate::{
    memory::ProcessMemoryView,
    process::{Module, Process},
    Previous, Result,
};

pub mod extensions;
pub mod log;
pub mod summary;
pub mod types;

use self::{
    summary::{GameSummary, InGameSummary},
    types::{
        BloonModel, BloonTargetProxy, GameModel, InGame, Tower, UpgradeModel, UpgradePathModel,
    },
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

    bail!("bloons process not found")
}

pub fn find_game_module(process: &Process) -> Result<Module> {
    for module in process.get_modules()? {
        let module_name = module.get_base_name()?;

        if module_name == "GameAssembly.dll" {
            return Ok(module);
        }
    }

    bail!("module not found")
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
    model_cache: &UpgradeModelCache,
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
    model_cache: &UpgradeModelCache,
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

#[derive(Default)]
struct BloonModelCacheBuilder {
    values: HashMap<String, BloonModelCacheEntry>,
}

impl BloonModelCacheBuilder {
    fn build(&mut self, raw: &BloonModel) -> Result<&BloonModelCacheEntry> {
        let id = raw.id()?.to_string();

        if !self.values.contains_key(&id) {
            let base_health = raw.max_health()? as u64;
            let mut base_total_rbe = base_health;
            let mut base_total_worth = 1;

            for child in raw.children()?.iter()? {
                let child = self.build(&child?)?;

                base_total_rbe += child.base_total_rbe;
                base_total_worth += child.base_total_worth;
            }

            self.values.insert(
                id.clone(),
                BloonModelCacheEntry {
                    base_health,
                    base_total_rbe,
                    base_total_worth,
                },
            );
        }

        Ok(&self.values[&id])
    }
}

pub struct BloonModelCache {
    values: HashMap<String, BloonModelCacheEntry>,
}

impl BloonModelCache {
    pub fn load(model: &GameModel) -> Result<Self> {
        let mut builder = BloonModelCacheBuilder::default();

        for bloon in model.bloons()?.iter()? {
            builder.build(&bloon?)?;
        }

        Ok(Self {
            values: builder.values,
        })
    }

    pub fn get(&self, id: impl AsRef<str>) -> Option<&BloonModelCacheEntry> {
        self.values.get(id.as_ref())
    }
}

#[derive(Clone)]
pub struct BloonModelCacheEntry {
    pub base_health: u64,

    pub base_total_rbe: u64,
    pub base_total_worth: u64,
}

impl BloonModelCacheEntry {
    pub fn worth(&self, round: u64) -> f32 {
        let multiplier = match round {
            ..=50 => 1.0,
            ..=60 => 0.5,
            ..=85 => 0.2,
            ..=100 => 0.1,
            ..=120 => 0.05,
            _ => 0.02,
        };

        self.base_total_worth as f32 * multiplier
    }
}

#[derive(Clone)]
pub struct UpgradeModelCache {
    upgrades: HashMap<String, UpgradeModel>,
}

impl UpgradeModelCache {
    pub fn load(model: &GameModel) -> Result<UpgradeModelCache> {
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

#[derive(Clone)]
pub struct BloonsGame {
    ingame_addr: Previous<u64>,
    model_cache: Option<UpgradeModelCache>,

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

    pub fn get_ingame(&self) -> Result<Option<InGame>> {
        InGame::get_instance(&self.memory, self.module_offset)
    }

    pub fn try_get_bloons(&self) -> Result<Option<BloonsState>> {
        match self.get_ingame()? {
            None => Ok(None),
            Some(ingame) => {
                let mut bloons = vec![];
                let mut max_path = 0.0f32;

                for path in ingame
                    .unity_to_simulation()?
                    .simulation()?
                    .map()?
                    .path_manager()?
                    .paths()?
                    .iter()?
                {
                    let path = path?;

                    for segment in path.segments()?.iter()? {
                        let segment = segment?;

                        max_path = max_path.max(segment.leak_distance()?);

                        for bloon in segment.bloons()?.iter()? {
                            let bloon = bloon?;

                            bloons.push(Bloon::load(bloon)?);
                        }
                    }
                }

                Ok(Some(BloonsState::new(bloons, max_path)))
            }
        }
    }

    pub fn get_summary(&mut self) -> GameSummary {
        let state = match self.try_get_summary() {
            Ok(s) => s,
            Err(e) => {
                let str = e.to_string();
                GameSummary::None(str)
            }
        };

        state
    }

    pub fn try_get_summary(&mut self) -> Result<GameSummary> {
        match self.get_ingame()? {
            None => Ok(GameSummary::None("null game".into())),
            Some(ingame) => {
                if self.ingame_addr.set(ingame.0.address) {
                    self.model_cache = None;
                }

                let model_cache = match self.model_cache.as_ref() {
                    Some(m) => m,
                    None => {
                        self.model_cache = Some(UpgradeModelCache::load(
                            &ingame.unity_to_simulation()?.simulation()?.model()?,
                        )?);
                        self.model_cache.as_ref().unwrap()
                    }
                };

                let state = InGameSummary::load(model_cache, &ingame)?;

                Ok(GameSummary::InGame(state))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BloonsState {
    pub bloons: Vec<Bloon>,
    pub max_path: f32,
}

impl BloonsState {
    pub fn new(bloons: Vec<Bloon>, max_path: f32) -> Self {
        Self { bloons, max_path }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bloon {
    pub kind: String,
    pub distance: f32,
}

impl Bloon {
    fn load(bloon: BloonTargetProxy) -> Result<Bloon> {
        let kind = bloon.bloon.model()?.base_id()?.to_string();
        let distance = bloon.bloon.distance_travelled()?;

        Ok(Self { kind, distance })
    }
}

pub struct BloonsHistogram {
    pub buckets: Vec<usize>,
    pub total: usize,
}

impl BloonsHistogram {
    pub fn new(bucket_count: usize) -> Self {
        let buckets = vec![0; bucket_count];
        let total = 0;

        Self { buckets, total }
    }

    pub fn add_one(&mut self, value: f32) {
        let index = (value * self.buckets.len() as f32) as usize;

        self.buckets[index] += 1;
        self.total += 1;
    }

    pub fn get_percentile(&self, value: f32) -> f32 {
        let index = (value * self.buckets.len() as f32) as usize;

        let better: usize = self.buckets[0..index].iter().sum();

        better as f32 / self.total as f32
    }
}
