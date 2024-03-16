use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::types::{ObjectId, Simulation};
use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GameLogState {
    pub label: String,
    pub seed: i32,
    pub time: u64,
    pub towers: HashMap<String, Tower>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tower {
    base_id: String,
    upgrades: HashSet<String>,
}

impl GameLogState {
    pub fn load(sim: &Simulation) -> Result<GameLogState> {
        let map_name = sim.model()?.map()?.map_name()?.to_string();
        let mode = sim.model()?.game_mode()?.to_string();

        let label = format!("{} {}", map_name, mode);
        let seed = sim.model()?.random_seed()?;
        let time = sim.time()?.elapsed()? as u64;

        let mut towers = HashMap::new();

        for tower in sim.map()?.towers()? {
            if tower.entity()?.is_some() && tower.parent_tower_id()? == ObjectId::INVALID {
                let id = tower.id()?.to_string();
                let base_id = tower.model()?.base_id()?.to_string();

                let mut upgrades = HashSet::new();

                for upgrade in tower.model()?.applied_upgrades()?.iter()? {
                    let upgrade = upgrade?;

                    upgrades.insert(upgrade.to_string());
                }

                towers.insert(id, Tower { base_id, upgrades });
            }
        }

        Ok(Self {
            label,
            seed,
            time,
            towers,
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameLog {
    entries: Vec<LogEntry>,
}

impl GameLog {
    pub fn update(&mut self, a: &GameLogState, b: &GameLogState) {
        self.entries.retain(|e| e.time <= b.time);

        for (tower_id, new_tower) in b.towers.iter() {
            match a.towers.get(tower_id) {
                Some(old_tower) => {
                    for upgrade_id in new_tower.upgrades.iter() {
                        if !old_tower.upgrades.contains(upgrade_id) {
                            self.entries.push(LogEntry {
                                time: b.time,
                                data: LogData::BuyUpgrade {
                                    tower_id: tower_id.clone(),
                                    upgrade_id: upgrade_id.clone(),
                                },
                            });
                        }
                    }
                }

                None => {
                    self.entries.push(LogEntry {
                        time: b.time,
                        data: LogData::BuyTower {
                            base_id: new_tower.base_id.clone(),
                            tower_id: tower_id.clone(),
                        },
                    });

                    for upgrade_id in new_tower.upgrades.iter() {
                        self.entries.push(LogEntry {
                            time: b.time,
                            data: LogData::BuyUpgrade {
                                tower_id: tower_id.clone(),
                                upgrade_id: upgrade_id.clone(),
                            },
                        });
                    }
                }
            }
        }

        for (tower_id, _) in a.towers.iter() {
            if let None = b.towers.get(tower_id) {
                self.entries.push(LogEntry {
                    time: b.time,
                    data: LogData::SellTower {
                        tower_id: tower_id.clone(),
                    },
                });
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub time: u64,
    pub data: LogData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogData {
    BuyTower {
        base_id: String,
        tower_id: String,
    },
    BuyUpgrade {
        tower_id: String,
        upgrade_id: String,
    },
    SellTower {
        tower_id: String,
    },
}
