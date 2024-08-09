use crate::{memory::ObjectPointer, Result};

use super::{
    types::{self, ObjectId, TowerSet, TowerToSimulation},
    UpgradeModelCache,
};

#[derive(Debug, Clone, PartialEq)]
pub enum GameSummary {
    None,
    InGame(InGameSummary),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InGameSummary {
    pub map_name: String,
    pub mode: String,
    pub seed: i32,

    pub cash: u64,
    pub danger: Option<f32>,
    pub max_path: f32,
    pub selected_index: Option<usize>,

    pub towers: Vec<Tower>,
    pub upgrades: Vec<Upgrade>,
}

impl InGameSummary {
    pub fn load(model_cache: &UpgradeModelCache, ingame: &types::InGame) -> Result<InGameSummary> {
        let cash = super::get_cash(ingame)?;

        let sim = ingame.unity_to_simulation()?.simulation()?;

        let map_name = sim.model()?.map()?.map_name()?.to_string();

        let mode = sim.model()?.game_mode()?.to_string();

        let seed = sim.model()?.random_seed()?;

        let mut towers = vec![];

        for tower in sim.map()?.towers()? {
            if tower.base().entity()?.is_some() && tower.parent_tower_id()? == ObjectId::INVALID {
                if tower.model()?.tower_set()? == TowerSet::HERO {
                    towers.push(Tower::Hero(Hero::load(&tower)?));
                } else {
                    towers.push(Tower::Basic(BasicTower::load(&tower)?));
                }
            }
        }

        let mut upgrades = vec![];

        for (tower, upgrade, _) in super::get_all_available_upgrades(model_cache, ingame)? {
            let id = tower.id()?.to_string();

            if let Some(index) = towers.iter().position(|t| match t {
                Tower::Basic(t) => t.id == id,
                _ => false,
            }) {
                upgrades.push(Upgrade::new(index, upgrade)?);
            }
        }

        let selected_index = match ingame.input_manager()?.selected()? {
            None => None,
            Some(selected) => {
                if let Ok(selected) = selected.cast::<TowerToSimulation>() {
                    let id = selected.tower()?.id()?.to_string();

                    towers.iter().position(|t| match t {
                        Tower::Basic(t) => t.id == id,
                        _ => false,
                    })
                } else {
                    None
                }
            }
        };

        let mut danger: Option<f32> = None;
        let mut max_path = 0.0f32;

        for path in sim.map()?.path_manager()?.paths()?.iter()? {
            let path = path?;

            for segment in path.segments()?.iter()? {
                let segment = segment?;

                max_path = max_path.max(segment.leak_distance()?);

                if segment.bloons()?.len()? > 0 {
                    match danger.as_mut() {
                        Some(danger) => *danger = danger.min(segment.leak_distance()?),
                        None => danger = Some(segment.leak_distance()?),
                    }
                }
            }
        }

        Ok(Self {
            map_name,
            mode,
            seed,
            cash,
            danger,
            max_path,
            selected_index,
            towers,
            upgrades,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tower {
    Basic(BasicTower),
    Hero(Hero),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hero {
    pub id: String,
    pub name: String,
    pub level: u8,
    pub worth: u64,
}

impl Hero {
    pub fn load(tower: &types::Tower) -> Result<Hero> {
        let id = tower.id()?.to_string();
        let name = tower.model()?.base_id()?.to_string();

        let level = tower.model()?.tier()? as u8;
        let worth = tower.worth()? as _;

        Ok(Self {
            id,
            name,
            level,
            worth,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicTower {
    pub id: String,
    pub name: String,
    pub tiers: [u8; 3],
    pub worth: u64,
}

impl BasicTower {
    pub fn load(tower: &types::Tower) -> Result<BasicTower> {
        let id = tower.id()?.to_string();
        let name = tower.model()?.base_id()?.to_string();
        let tiers = tower
            .model()?
            .tiers()?
            .iter()?
            .map(|v| v.map(|v| v as u8))
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .expect("3 tiers");

        let worth = tower.worth()? as _;

        Ok(Self {
            id,
            name,
            tiers,
            worth,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upgrade {
    pub tower_index: usize,
    pub path: usize,
    pub tier: u8,
    pub cost: u64,
    pub name: String,
}

impl Upgrade {
    fn new(tower_index: usize, upgrade: types::UpgradeModel) -> Result<Upgrade> {
        let path = upgrade.path()?.try_into()?;
        let tier = upgrade.tier()?.try_into()?;
        let cost = upgrade.cost()?.try_into()?;
        let name = upgrade.base().name()?.to_string();

        Ok(Self {
            tower_index,
            path,
            tier,
            cost,
            name,
        })
    }
}
