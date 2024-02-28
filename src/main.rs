use app::App;
use windows::Win32::System::Diagnostics::Debug::Beep;

mod app;
mod btd;
mod memory;
mod process;

use process::Process;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// #[derive(Debug, Clone, Copy)]
// enum Condition {
//     Cash(u64),
//     Round(u64),
// }

// impl Condition {
//     pub fn check(&self, ingame: &InGame) -> Result<bool> {
//         println!("waiting: {:?}", self);

//         let simulation = ingame.unity_to_simulation()?.simulation()?;

//         let round = simulation.map()?.spawner()?.current_round()?.get()? as u64 + 1;
//         let cash = simulation.cash_manager()?.cash()?.get()? as u64;

//         let met = match self {
//             &Condition::Cash(v) => cash >= v,
//             &Condition::Round(v) => round >= v,
//         };

//         Ok(met)
//     }
// }

// impl FromStr for Condition {
//     type Err = Box<dyn std::error::Error>;

//     fn from_str(s: &str) -> Result<Condition> {
//         let target_value = s.parse()?;

//         if target_value > 100 {
//             Ok(Self::Cash(target_value))
//         } else {
//             Ok(Self::Round(target_value))
//         }
//     }
// }

fn beep() {
    unsafe {
        Beep(500, 200).unwrap();
    }
}

struct Previous<T> {
    value: Option<T>,
}

impl<T> Default for Previous<T> {
    fn default() -> Self {
        Self { value: None }
    }
}

impl<T: PartialEq> Previous<T> {
    pub fn set(&mut self, value: T) -> bool {
        let is_update = match &self.value {
            Some(v) => *v != value,
            None => true,
        };

        self.value = Some(value);
        is_update
    }
}

// struct Helper<'a> {
//     memory: &'a ProcessMemoryView,
//     module: &'a Module<'a>,

//     cash: u64,

//     model_cache: Option<ModelCache>,
//     ingame_address: Previous<u64>,

//     waiting_for: Previous<String>,
//     summary: Previous<InGameState>,
// }

// impl<'a> Helper<'a> {
//     fn new(memory: &'a ProcessMemoryView, module: &'a Module<'a>) -> Self {
//         let cash = 0;
//         let model_cache = None;
//         let ingame_address = Default::default();
//         let waiting_for = Default::default();
//         let summary = Default::default();

//         Self {
//             memory,
//             module,
//             cash,
//             model_cache,
//             ingame_address,
//             waiting_for,
//             summary,
//         }
//     }

//     pub fn tick(&mut self) -> Result<()> {
//         let ingame = match InGame::get_instance(&self.memory, self.module.get_bounds()?.0)? {
//             Some(v) => v,
//             None => return Err("not in game".into()),
//         };

//         if self.ingame_address.set(ingame.0.address) {
//             self.model_cache = None;

//             println!("new game detected");

//             // fn test(indent: usize, node: &Entity) -> Result<()> {
//             //     unsafe {
//             //         // println!("{}{}", "  ".repeat(indent), node.get_type()?.get_name()?);
//             //         // println!(
//             //         //     "{}{:?}",
//             //         //     "  ".repeat(indent + 1),
//             //         //     node.field::<Option<Object>>(0x58)?
//             //         //         .map(|v| v.get_type().and_then(|v| v.get_name()))
//             //         // );

//             //         for
//             //             for o in list.list()?.iter()? {
//             //                 let o = o?;

//             //                 println!("{}{}", "  ".repeat(indent + 1), o.get_type()?.get_name()?);
//             //             }

//             //         // println!("{}--", "  ".repeat(indent + 1));

//             //         // let dependants: Option<LockList<Object>> = node.field(0x60)?;
//             //         // if let Some(list) = dependants {
//             //         //     for o in list.list()?.iter()? {
//             //         //         println!("{}{}", "  ".repeat(indent + 1), o?.get_type()?.get_name()?);
//             //         //         // test(indent + 1, &o?)?;
//             //         //     }
//             //         // }

//             //         println!("{}--", "  ".repeat(indent + 1));

//             //         let dependant_entities: Option<LockList<Entity>> = node.field(0x68)?;
//             //         if let Some(list) = dependant_entities {
//             //             for o in list.list()?.iter()? {
//             //                 test(indent + 1, &o?)?;
//             //             }
//             //         }

//             //         Ok(())
//             //     }
//             // }

//             // test(0, &ingame.unity_to_simulation()?.simulation()?.entity()?)?;

//             // test(
//             //     0,
//             //     &ingame
//             //         .unity_to_simulation()?
//             //         .simulation()?
//             //         .map()?
//             //         .entity()?
//             //         .into(),
//             // )?;

//             for path in ingame
//                 .unity_to_simulation()?
//                 .simulation()?
//                 .map()?
//                 .path_manager()?
//                 .paths()?
//                 .iter()?
//             {
//                 let path = path?;

//                 println!(
//                     "path {} {:?}",
//                     path.leak_distance()?,
//                     path.best_first_all()?
//                         .map(|b| b.model().and_then(|v| v.base_id()).map(|v| v.to_string()))
//                         .transpose()?,
//                 );

//                 for bloon in path.bloons()?.list()?.iter()? {
//                     let bloon = bloon?;

//                     println!(
//                         "    {} {}",
//                         bloon.model()?.base_id()?,
//                         bloon.distance_travelled()?
//                     );
//                 }
//             }

//             // println!("{}", &unsafe {
//             //     ingame
//             //         .unity_to_simulation()?
//             //         .simulation()?
//             //         .map()?
//             //         .entity()?
//             //         .dependants()?
//             //         .list()?
//             //         .get_type()?
//             //         .get_name()?
//             // });
//             // test(&unsafe {
//             //     ingame
//             //         .unity_to_simulation()?
//             //         .simulation()?
//             //         .map()?
//             //         .field::<Object>(0x40)?
//             // })?;

//             // for tower in ingame.unity_to_simulation().simulation().map().towers() {
//             //     println!("{} {}", tower.model().base_id(), tower.worth());
//             // }
//             // for bloon_model in ingame
//             //     .unity_to_simulation()
//             //     .simulation()
//             //     .model()
//             //     .bloons()
//             //     .iter()
//             // {
//             //     println!(
//             //         "{} {} {}",
//             //         bloon_model.id(),
//             //         bloon_model.get_worth(),
//             //         bloon_model.children().len()
//             //     );
//             // }

//             // for (k, v) in ingame.unity_to_simulation().simulation().map().towers_by_area().iter() {
//             //     println!("{:?} {:?}", k, v);
//             // }

//             // for (k, v) in ingame.unity_to_simulation().simulation().map().spawner().round_data().iter() {
//             //     println!("{:?} {:?}", k, v);
//             // }
//             // println!("{:?}", ingame.unity_to_simulation().simulation().map().spawner().round_data().get_type().get_name());
//         }

//         let model_cache = match self.model_cache.as_ref() {
//             Some(m) => m,
//             None => {
//                 self.model_cache = Some(ModelCache::load(
//                     &ingame.unity_to_simulation()?.simulation()?.model()?,
//                 )?);
//                 self.model_cache.as_ref().unwrap()
//             }
//         };

//         if self.summary.set(InGameState::load(&ingame)?) {
//             self.summary.value.as_ref().unwrap().print();
//         }

//         let selected = ingame.input_manager()?.selected()?;

//         let available = match selected {
//             Some(tower) => btd::get_available_upgrades(&model_cache, &tower.tower()?)?,
//             None => btd::get_all_available_upgrades(&model_cache, &ingame)?,
//         };

//         let new_cash = btd::get_cash(&ingame)?;

//         let do_beep = available
//             .iter()
//             .any(|(_, _, cost)| (self.cash..new_cash).contains(&cost));

//         if do_beep {
//             beep();
//         } else {
//             let next = available.iter().find(|(_, _, cost)| *cost > new_cash);

//             if let Some((_, upgrade, cost)) = next {
//                 if self.waiting_for.set(upgrade.name()?.to_string()) {
//                     println!("Waiting for {} ${}", upgrade.name()?, cost);
//                 }
//             }
//         }

//         self.cash = new_cash;

//         Ok(())
//     }
// }

fn main() -> Result<()> {
    // let debug = BloonsGame::find_game()?;
    // let state = debug.try_get_state();

    // println!("{:#?}", state);

    let mut app = App::new();
    app.run()?;

    // // let conditions = std::env::args()
    // //     .skip(1)
    // //     .map(|s| s.parse())
    // //     .collect::<Result<Vec<_>>>()?;

    // let pid = btd::find_pid()?;

    // let process = Process::from_pid(pid, PROCESS_QUERY_INFORMATION | PROCESS_VM_READ)?;

    // let memory = ProcessMemoryView::new(process);
    // let module = btd::find_game_module(&process)?;

    // // for condition in conditions {
    // //     check(&ingame, condition)?;
    // //     beep();
    // // }

    // let mut helper = Helper::new(&memory, &module);

    // loop {
    //     if let Err(e) = helper.tick() {
    //         println!("{:?}", e);
    //     }

    //     sleep(Duration::from_millis(100));
    // }

    Ok(())
}
