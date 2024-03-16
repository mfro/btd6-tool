use super::types::{self, CSharpString, TowerModel};
use crate::{memory::ObjectPointer, Result, TryMap};

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    None,
    InGame(InGameState),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InGameState {
    pub model: GameModel,

    pub paths: Vec<Path>,

    pub cash: u64,
    pub round: u64,
    pub lives: u64,

    pub towers: Vec<Tower>,
    pub selected_tower: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameModel {
    pub map_name: String,
    pub game_mode: String,
    pub seed: i32,

    pub upgrades: UpgradeLookup,
    pub towers: TowerLookup,
}

impl GameModel {
    pub fn load(model: &types::GameModel) -> Result<Self> {
        let seed = model.random_seed()?;
        let map_name = model.map()?.map_name()?.try_into()?;
        let game_mode = model.game_mode()?.try_into()?;

        let t0 = std::time::Instant::now();
        let upgrades = UpgradeLookup::load(model)?;
        println!("  {:?}", t0.elapsed());
        let t0 = std::time::Instant::now();
        let towers = TowerLookup::load(model, &upgrades)?;
        println!("  {:?}", t0.elapsed());

        Ok(Self {
            seed,
            map_name,
            game_mode,
            towers,
            upgrades,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpgradeLookup {
    pub upgrades: Vec<Upgrade>,
}

impl UpgradeLookup {
    pub fn load(model: &types::GameModel) -> Result<Self> {
        let upgrades = model
            .upgrades()?
            .iter()?
            .and_then_map(|v| Upgrade::load(&v))
            .collect::<Result<_>>()?;

        Ok(Self { upgrades })
    }

    pub fn by_id(&self, id: CSharpString) -> Result<usize> {
        let id: String = id.try_into()?;

        match self.upgrades.iter().position(|v| v.id == id) {
            Some(v) => Ok(v),
            None => Err(format!("unknown upgrade: {}", id).into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Upgrade {
    pub id: String,
    pub path: i8,
    pub tier: u8,
    pub cost: u64,
}

impl Upgrade {
    pub fn load(model: &types::UpgradeModel) -> Result<Self> {
        let id: String = model.name()?.try_into()?;

        let upgrade = Upgrade {
            id,
            path: model.path()?.try_into()?,
            tier: model.tier()?.try_into()?,
            cost: model.cost()?.try_into()?,
        };

        Ok(upgrade)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TowerLookup {
    pub towers: Vec<TowerKind>,
}

impl TowerLookup {
    pub fn load(model: &types::GameModel, upgrades: &UpgradeLookup) -> Result<Self> {
        let towers = model
            .towers()?
            .iter()?
            .and_then_map(|v| TowerKind::load(&v, upgrades))
            .collect::<Result<_>>()?;

        Ok(Self { towers })
    }

    pub fn by_id(&self, id: CSharpString) -> Result<usize> {
        let id: String = id.try_into()?;

        match self.towers.iter().position(|v| v.id == id) {
            Some(v) => Ok(v),
            None => Err(format!("unknown upgrade: {}", id).into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TowerKind {
    pub id: String,
    pub set: u32,
    pub tiers: [u8; 3],
    pub applied_upgrades: Vec<usize>,
    pub available_upgrades: Vec<usize>,
}

impl TowerKind {
    pub fn load(model: &TowerModel, upgrades: &UpgradeLookup) -> Result<Self> {
        let tiers = model
            .tiers()?
            .iter()?
            .and_then_map(|v| Ok(v as u8))
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .map_err(|_| "3 tiers")?;

        let applied_upgrades = model
            .applied_upgrades()?
            .iter()?
            .and_then_map(|v: CSharpString| Ok(upgrades.by_id(v)?))
            .collect::<Result<Vec<_>>>()?;

        let available_upgrades = model
            .upgrades()?
            .iter()?
            .and_then_map(|v| Ok(upgrades.by_id(v.upgrade()?)?))
            .collect::<Result<Vec<_>>>()?;

        Ok(TowerKind {
            id: model.base_id()?.try_into()?,
            set: model.tower_set()?,
            tiers,
            applied_upgrades,
            available_upgrades,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub bloons: Vec<Bloon>,
    pub leak_distance: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bloon {
    pub kind: String,
    pub distance: f32,
    pub leak_distance: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tower {
    pub id: String,
    pub kind: usize,
    pub worth: f32,
}

impl InGameState {
    pub fn load(ingame: &types::InGame) -> Result<InGameState> {
        let sim = ingame.unity_to_simulation()?.simulation()?;

        let model = GameModel::load(&sim.model()?)?;

        let mut paths = vec![];
        for path in sim.map()?.path_manager()?.paths()?.iter()? {
            let path = path?;

            let mut bloons = vec![];
            for segment in path.segments()?.iter()? {
                let segment = segment?;

                for bloon in segment.bloons()?.iter()? {
                    let bloon = bloon?.bloon;

                    bloons.push(Bloon {
                        kind: bloon.model()?.base_id()?.try_into()?,
                        distance: bloon.distance_travelled()?,
                        leak_distance: segment.leak_distance()?,
                    });
                }
            }
            paths.push(Path {
                bloons,
                leak_distance: path.leak_distance()?,
            });
        }

        let cash = sim.cash_manager()?.cash()?.get()? as _;
        let round = sim.map()?.spawner()?.current_round()?.get()? as _;
        let lives = sim.health()?.get()? as _;

        let mut towers = vec![];
        for tower in sim.map()?.towers()? {
            if tower.entity()?.is_some() && tower.parent_tower_id()? == types::ObjectId::INVALID {
                towers.push(Tower {
                    id: tower.id()?.try_into()?,
                    kind: model.towers.by_id(tower.model()?.base_id()?)?,
                    worth: tower.worth()?,
                });
            }
        }

        let selected_tower = match ingame.input_manager()?.selected()? {
            None => None,
            Some(selected) => {
                if let Ok(selected) = selected.cast::<types::TowerToSimulation>() {
                    let id = selected.tower()?.id()?.to_string();

                    towers.iter().position(|t| t.id == id)
                } else {
                    None
                }
            }
        };

        Ok(Self {
            model,
            paths,
            cash,
            round,
            lives,
            towers,
            selected_tower,
        })
    }
}
