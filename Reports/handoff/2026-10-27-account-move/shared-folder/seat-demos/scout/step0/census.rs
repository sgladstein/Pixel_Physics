// Scout demo step 0a: does a trained colony stay alive but hungry with a near trickle and a far heap?
// args: trickle_cells_per_1000 seed frames [far_x] [bank=1]
use pixel_physics::lab::scenario::{Event, Placement, Scenario};
use pixel_physics::lab::Lab;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let trickle: i32 = a[0].parse().unwrap();
    let seed: u64 = a[1].parse().unwrap();
    let frames: u64 = a[2].parse().unwrap();
    let far_x: i32 = a.get(3).and_then(|v| v.parse().ok()).unwrap_or(330);
    let bank = a.get(4).map_or(true, |v| v != "0");
    let mut sc = Scenario::load("steady_income").unwrap();
    sc.bed.seed = seed;
    sc.timeline.clear();
    sc.timeline.push(Event { at: std::env::var("LAND").ok().and_then(|v| v.parse().ok()).unwrap_or(200), every: 0, until: 0, what: Placement::Colony { species: "ant".into(), x: 128, count: 52 } });
    if trickle > 0 {
        sc.timeline.push(Event { at: 200, every: 1000, until: 0, what: Placement::Heap { material: "provisions".into(), x: 153, cells: trickle } });
    }
    sc.timeline.push(Event { at: 200, every: 6000, until: 0, what: Placement::Heap { material: "provisions".into(), x: far_x, cells: 120 } });
    if bank {
        sc.placements.push(Placement::Fill { material: "soil".into(), x: 236, y: 154, w: 14, h: 6 });
    }
    let mut lab = Lab::new(sc.bed.clone());
    lab.load_scenario(sc);
    lab.set_new_ant(true);
    let ant = lab.world.species.id_of("ant").unwrap();
    let def = lab.world.species.creature_shared(ant).unwrap();
    let tank = def.sensed_tank.unwrap_or(def.start_energy);
    let mut line = format!("trickle={trickle} seed={seed} far_x={far_x} bank={bank}:");
    for f in 1..=frames {
        lab.tick_for_harness();
        if std::env::var("EARLY").is_ok() && f % 500 == 0 && f <= std::env::var("EARLY").unwrap().parse::<u64>().unwrap() {
            let n = lab.world.live_organism_ids().into_iter().filter(|&i| lab.world.organism(i).is_some_and(|s| s.species == ant)).count();
            eprintln!("f{f} alive {n} deaths {:?}", lab.world.deaths_by_cause);
        }
        if f % 6000 == 0 {
            let ids: Vec<_> = lab.world.live_organism_ids().into_iter().filter(|&i| lab.world.organism(i).is_some_and(|s| s.species == ant)).collect();
            let fr: Vec<f32> = ids.iter().map(|&i| lab.world.organism(i).unwrap().energy / tank).collect();
            let hungry = fr.iter().filter(|&&e| e < 0.5).count();
            let mut s = fr.clone(); s.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let med = if s.is_empty() { 0.0 } else { s[s.len() / 2] };
            let far = ids.iter().filter(|&&i| lab.world.organism(i).unwrap().chain.first().is_some_and(|&(x, _)| (x - far_x).abs() < 20)).count();
            line += &format!(" f{}k={} h{} m{:.2} far{}", f / 1000, ids.len(), hungry, med, far);
        }
    }
    println!("{line}");
}
