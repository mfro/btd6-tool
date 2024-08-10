use core::panic;
use std::{
    cell::RefCell, collections::HashMap, fs::File, io::{Read, Write}
};

use anyhow::Context;
use assert_matches::assert_matches;
use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::quote;
use regex::Regex;
use syn::{punctuated::Punctuated, Expr, Lit, Meta};

type Result<T> = anyhow::Result<T>;

const DUMP: &'static str = include_str!("../dump.cs");

struct BindingGenerator {
    dump: String,
}

impl BindingGenerator {
    fn new(dump: String) -> Self {
        Self { dump }
    }

    fn get_class(&self, namespace: &str, name: &str) -> Result<ClassBindingGenerator> {
        let regex = Regex::new(&format!(
            r"// Namespace: {}\n(\[.*\n)*.*class {} .*\n\{{\n((\n|\s+.+\n)*)\}}",
            regex::escape(&namespace),
            regex::escape(&name),
        ))?;

        let captures = regex.captures(&self.dump).context("class not found")?;
        let body = captures.get(2).unwrap().as_str();

        Ok(ClassBindingGenerator { body })
    }
}

struct ClassBindingGenerator<'a> {
    body: &'a str,
}

impl<'a> ClassBindingGenerator<'a> {
    fn get_field_offset(&self, field_name: &str) -> Result<usize> {
        let regex = Regex::new(&format!(
            r"\s([^\s]+) {}; // 0x([A-Z\d]+)\n",
            regex::escape(field_name),
        ))?;

        let captures = regex.captures(&self.body).context("field not found")?;
        let raw_value = captures.get(2).unwrap().as_str();

        let value = usize::from_str_radix(raw_value, 16)?;

        // offsets from il2cpp include the constant-sized class offset of 16 bytes
        Ok(value - 16)
    }
}

struct ClassBinding {
    super_type: Option<String>,
    bind_namespace: String,
    bind_name: String,
    name: String,
    fields: Vec<FieldBinding>,
}

struct FieldBinding {
    bind_name: String,
    name: String,
    ty: String,
}

impl ClassBinding {
    fn new(fullname: &str) -> Self {
        let mut parts = fullname.split('.').collect::<Vec<_>>();
        let name = parts.pop().unwrap().to_owned();
        let namespace = parts.join(".");

        Self {
            bind_namespace: namespace,
            bind_name: name.clone(),
            name,
            fields: vec![],
            super_type: None,
        }
    }

    fn add_field(&mut self, field: FieldBinding) {
        self.fields.push(field)
    }

    fn extend(&mut self, base: &ClassBinding) {
        self.super_type = Some(base.name.clone());
    }

    fn generate(&self, gen: &BindingGenerator, out: &mut impl Write) -> Result<()> {
        let class = gen.get_class(&self.bind_namespace, &self.bind_name)?;

        writeln!(out, "object_type!({});", self.name)?;
        writeln!(out, "impl {} {{", self.name)?;

        if let Some(super_type) = self.super_type.as_ref() {
            writeln!(out, "  super_type!({});", super_type)?;
        }

        for field in self.fields.iter() {
            let value = class.get_field_offset(&field.bind_name)?;

            writeln!(
                out,
                "  field!(0x{value:0>4x} {}: {});",
                field.name, field.ty
            )?;
        }

        writeln!(out, "}}")?;

        Ok(())
    }
}

macro_rules! variable_declaration {
    ( $pop:ident. $( $rest:ident ).* ) => {
        variable_declaration!($( $rest ).*)
    };
    ( $last:ident ) => {
        mut $last
    };
}

macro_rules! class {
    ($( $full_name:ident).* ) => {
        let class_binding = ClassBinding::new(stringify!( $( $full_name ).* ));
        #[allow(unused_mut)]
        let variable_declaration!($( $full_name ).*) = class_binding;
    };
}

macro_rules! field {
    ($class: expr, $name:ident: $ty:ty as $bind_name:literal) => {
        $class.add_field(FieldBinding {
            bind_name: $bind_name.to_owned(),
            name: stringify!($name).to_owned(),
            ty: stringify!($ty).to_owned(),
        });
    };
}

fn get_bindgen() -> Result<BindingGenerator> {
    let bindgen = BindingGenerator::new(DUMP.to_owned());

    Ok(bindgen)
}

#[allow(non_snake_case, unused_variables)]
fn main() -> Result<()> {
    let bindgen = get_bindgen();

    let mut out = File::create("../btd6-tool/src/btd/types_generated.rs")?;

    writeln!(out, "#![allow(non_camel_case_types, dead_code)]")?;
    writeln!(out, "use crate::memory::*;")?;
    writeln!(out, "use super::types::*;")?;

    class!(Assets.Scripts.Unity.UI_New.InGame.InGame);
    field!(InGame, input_manager: InputManager as "inputManager");
    field!(InGame, unity_to_simulation: UnityToSimulation as "bridge");
    field!(InGame, stopped_clock_for_menu_open: bool as "stoppedClockForMenuOpen");
    InGame.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Unity.UI_New.InGame.InputManager);
    field!(InputManager, selected: Option<Object> as "selected");
    InputManager.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Unity.Bridge.UnityToSimulation);
    field!(UnityToSimulation, simulation: Simulation as "simulation");
    UnityToSimulation.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Unity.Bridge.TowerToSimulation);
    field!(TowerToSimulation, tower: Tower as "tower");
    TowerToSimulation.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Simulation);
    field!(Simulation, entity: Object as "entity");
    field!(Simulation, model: GameModel as "model");
    field!(Simulation, time: Time as "time");
    field!(Simulation, round_time: Time as "roundTime");
    field!(Simulation, tower_manager: TowerManager as "towerManager");
    field!(Simulation, cash_managers: Dictionary<Object, CashManager> as "cashManagers");
    field!(Simulation, health: KonFuze as "health");
    field!(Simulation, map: Map as "map");
    Simulation.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Time);
    field!(Time, elapsed: i32 as "elapsed");
    Time.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Objects.Entity);
    field!(Entity, dependants: LockList<RootObject> as "dependants");
    Entity.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Objects.RootObject);
    field!(RootObject, id: CSharpString as "Id");
    RootObject.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Objects.RootBehavior);
    RootBehavior.extend(&RootObject);
    field!(RootBehavior, entity: Option<Entity> as "entity");
    RootBehavior.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Towers.TowerManager);
    TowerManager.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Track.Map);
    Map.extend(&RootBehavior);
    field!(Map, model: MapModel as "mapModel");
    field!(Map, path_manager: PathManager as "pathManager");
    field!(Map, spawner: Spawner as "spawner");
    field!(Map, towers_by_area: Dictionary<Pointer, List<Tower>> as "areaTowers");
    Map.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Track.PathManager);
    field!(PathManager, paths: List<Path> as "paths");
    PathManager.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Track.Path);
    field!(Path, segments: Array<PathSegment> as "segments");
    field!(Path, model: PathModel as "def");
    field!(Path, is_active: bool as "isActive");
    field!(Path, is_hidden: bool as "isHidden");
    field!(Path, spawn_distance: f32 as "spawnDist");
    field!(Path, leak_distance: f32 as "leakDist");
    field!(Path, bloons: LockList<Bloon> as "bloonsList");
    field!(Path, total_path_length: f32 as "totalPathLength");
    Path.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Track.PathSegment);
    field!(PathSegment, bloons: List<BloonTargetProxy> as "bloons");
    field!(PathSegment, min: f32 as "min");
    field!(PathSegment, max: f32 as "max");
    field!(PathSegment, center: f32 as "centre");
    field!(PathSegment, leak_distance: f32 as "distanceUntilLeak");
    PathSegment.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Track.Spawner);
    field!(Spawner, round_data: Dictionary<u32, RoundData> as "roundData");
    field!(Spawner, current_round: KonFuze_NoShuffle as "currentRound");
    Spawner.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Track.RoundData);
    RoundData.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Models.Map.PathModel);
    PathModel.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Towers.Tower);
    Tower.extend(&RootBehavior);
    field!(Tower, id: CSharpString as "uniqueId");
    field!(Tower, worth: f32 as "worth");
    field!(Tower, damage_dealt: u64 as "damageDealt");
    field!(Tower, cash_earned: u64 as "cashEarned");
    field!(Tower, applied_cash: f32 as "appliedCash");
    field!(Tower, parent_tower_id: ObjectId as "parentTowerId");
    field!(Tower, model: TowerModel as "towerModel");
    Tower.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Simulation.Bloons.Bloon);
    field!(Bloon, model: BloonModel as "bloonModel");
    field!(Bloon, distance_travelled: f32 as "distanceTraveled");
    Bloon.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Models.Model);
    field!(Model, name: CSharpString as "_name");
    Model.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Models.GameModel);
    field!(GameModel, difficulty_id: CSharpString as "difficultyId");
    field!(GameModel, game_type: CSharpString as "gameType");
    field!(GameModel, game_mode: CSharpString as "gameMode");
    field!(GameModel, random_seed: i32 as "randomSeed");
    field!(GameModel, reverse_mode: bool as "reverseMode");
    field!(GameModel, map: MapModel as "map");
    field!(GameModel, round_set: RoundSetModel as "<roundSet>k__BackingField");
    field!(GameModel, income_set: IncomeSetModel as "<incomeSet>k__BackingField");
    field!(GameModel, towers: Array<TowerModel> as "towers");
    field!(GameModel, upgrades: Array<UpgradeModel> as "upgrades");
    field!(GameModel, bloons: Array<BloonModel> as "bloons");
    GameModel.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Models.Map.MapModel);
    field!(MapModel, map_difficulty: i32 as "mapDifficulty");
    field!(MapModel, map_name: CSharpString as "mapName");
    MapModel.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Models.Rounds.RoundModel);
    field!(RoundModel, groups: Array<BloonGroupModel> as "groups");
    field!(RoundModel, emissions: Option<Array<BloonEmissionModel>> as "emissions_");
    RoundModel.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Models.Rounds.RoundSetModel);
    field!(RoundSetModel, rounds: Array<RoundModel> as "rounds");
    RoundSetModel.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Models.Rounds.BloonGroupModel);
    field!(BloonGroupModel, bloon: CSharpString as "bloon");
    field!(BloonGroupModel, start: f32 as "start");
    field!(BloonGroupModel, end: f32 as "end");
    field!(BloonGroupModel, count: i32 as "count");
    BloonGroupModel.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Models.Rounds.BloonEmissionModel);
    field!(BloonEmissionModel, bloon: CSharpString as "bloon");
    field!(BloonEmissionModel, time: f32 as "time");
    field!(BloonEmissionModel, emission_index: i32 as "emissionIndex");
    field!(BloonEmissionModel, is_custom_boss_emission: bool as "isCustomBossEmission");
    field!(BloonEmissionModel, tower_set_immunity: u32 as "towerSetImmunity");
    BloonEmissionModel.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Models.Rounds.IncomeSetModel);
    IncomeSetModel.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Models.Bloons.BloonModel);
    field!(BloonModel, id: CSharpString as "id");
    field!(BloonModel, base_id: CSharpString as "baseId");
    field!(BloonModel, max_health: i32 as "maxHealth");
    field!(BloonModel, leak_damage: f32 as "leakDamage");
    field!(BloonModel, layer_number: i32 as "layerNumber");
    field!(BloonModel, children: List<BloonModel> as "childBloonModels");
    BloonModel.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Models.Towers.TowerModel);
    field!(TowerModel, base_id: CSharpString as "baseId");
    field!(TowerModel, tier: u32 as "tier");
    field!(TowerModel, tiers: Array<u32> as "tiers");
    field!(TowerModel, tower_set: u32 as "towerSet");
    field!(TowerModel, upgrades: Array<UpgradePathModel> as "upgrades");
    field!(TowerModel, applied_upgrades: Array<CSharpString> as "appliedUpgrades");
    field!(TowerModel, is_bakable: bool as "isBakable");
    TowerModel.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Models.Towers.Upgrades.UpgradeModel);
    UpgradeModel.extend(&Model);
    field!(UpgradeModel, cost: i32 as "cost");
    field!(UpgradeModel, xp_cost: i32 as "xpCost");
    field!(UpgradeModel, path: i32 as "path");
    field!(UpgradeModel, tier: i32 as "tier");
    field!(UpgradeModel, locked: i32 as "locked");
    UpgradeModel.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Models.Towers.Upgrades.UpgradePathModel);
    field!(UpgradePathModel, tower: CSharpString as "tower");
    field!(UpgradePathModel, upgrade: CSharpString as "upgrade");
    UpgradePathModel.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Utils.KonFuze);
    field!(KonFuze, get: f64 as "honey");
    KonFuze.generate(&bindgen, &mut out)?;

    class!(Assets.Scripts.Utils.KonFuze_NoShuffle);
    KonFuze_NoShuffle.extend(&KonFuze);
    KonFuze_NoShuffle.generate(&bindgen, &mut out)?;

    Ok(())
}
