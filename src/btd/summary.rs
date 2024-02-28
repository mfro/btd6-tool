use std::fmt::Display;

use crate::Result;

use super::{types, ModelCache};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameState {
    None,
    InGame(InGameState),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InGameState {
    pub cash: u64,
    pub selected_index: Option<usize>,

    pub towers: Vec<Tower>,
    pub upgrades: Vec<Upgrade>,
}

impl InGameState {
    pub fn load(model_cache: &ModelCache, ingame: &types::InGame) -> Result<InGameState> {
        let cash = super::get_cash(ingame)?;

        let mut towers = vec![];

        for tower in ingame
            .unity_to_simulation()?
            .simulation()?
            .map()?
            .towers()?
        {
            if tower.entity()?.is_some() && tower.worth()? > 0.0 {
                towers.push(Tower::load(&tower)?);
            }
        }

        let mut upgrades = vec![];

        for (tower, upgrade, _) in super::get_all_available_upgrades(model_cache, ingame)? {
            let id = tower.id()?.to_string();

            if towers.iter().any(|t| t.id == id) {
                upgrades.push(Upgrade::new(&towers, tower, upgrade)?);
            }
        }

        let selected_index = match ingame.input_manager()?.selected()? {
            None => None,
            Some(s) => {
                let id = s.tower()?.id()?.to_string();
                towers.iter().position(|t| t.id == id)
            }
        };

        Ok(Self {
            cash,
            selected_index,
            towers,
            upgrades,
        })
    }

    pub fn print(&self) {
        println!(
            "total: ${}",
            self.towers.iter().map(|t| t.worth).sum::<u64>()
        );

        for tower in self.towers.iter() {
            println!("  {}", tower);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tower {
    pub id: String,
    pub name: String,
    pub tiers: [u8; 3],
    pub worth: u64,
}

impl Tower {
    pub fn load(tower: &types::Tower) -> Result<Tower> {
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

impl Display for Tower {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}-{}-{} {} ${}",
            self.tiers[0], self.tiers[1], self.tiers[2], self.name, self.worth
        )
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
    fn new(towers: &[Tower], tower: types::Tower, upgrade: types::UpgradeModel) -> Result<Upgrade> {
        let id = tower.id()?.to_string();
        let tower_index = match towers.iter().position(|t| t.id == id) {
            Some(v) => v,
            None => return Err("tower not found".into()),
        };

        let path = upgrade.path()? as _;
        let tier = upgrade.tier()? as _;
        let cost = upgrade.cost()? as _;
        let name = upgrade.name()?.to_string();

        Ok(Self {
            tower_index,
            path,
            tier,
            cost,
            name,
        })
    }
}
