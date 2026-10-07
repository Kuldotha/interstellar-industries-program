#![allow(dead_code)]
include!("shared.rs");

fn capacities(groups: &[(usize, u64)]) -> [u64; N] {
    let mut cap = [0; N];
    for &(i, n) in groups {
        cap[i] = n * Q;
    }
    cap
}
fn baseline(cap: &[u64; N], stocks: &[u64; R]) -> [u64; WORDS] {
    let mut state = [0; WORDS];
    state[..N].copy_from_slice(cap);
    state[AVAILABLE] = 65536 * Q;
    for r in 0..R {
        state[stock_index(r)] = stocks[r];
    }
    prepare_stock(&mut state).unwrap();
    let change = prepare(&mut state, 255).unwrap();
    execute(&mut state, change).unwrap();
    state
}
fn unused(cap: &[u64; N], stock: &[u64; R], up: &active::AppliedSweep) -> Option<(usize, u64)> {
    for i in 0..N {
        if RECIPE_RESOURCES[i].len() == 1 {
            continue;
        }
        let mut increase = cap[i] - up.production[i];
        for &r in &RECIPE_RESOURCES[i][1..] {
            if stock[r] == 0 {
                increase = increase.min(up.balance[r].max(0) as u64);
            }
        }
        if increase > Q / 10000 {
            return Some((i, increase));
        }
    }
    None
}
fn main() {
    let cap = capacities(&[(30, 10), (31, 1), (32, 5), (51, 10), (77, 10)]);
    let down = active::collect_demand(&cap);
    let up = active::apply_demand(&cap, &[0; R], &down);
    assert_eq!(up.production[32], Q);
    assert_eq!(up.production[77], 9 * Q);
    println!("PASS known local bottleneck: Electronics 1, Superconductors 9");

    let cap = capacities(&[(30, 1), (31, 1), (32, 5), (51, 10), (77, 10)]);
    let down = active::collect_demand(&cap);
    let up = active::apply_demand(&cap, &[0; R], &down);
    assert_eq!(down.no_stock_ceiling[32], Q);
    assert!(up.production[32] < Q);
    println!(
        "PASS upward finalizes preliminary ceiling: Electronics {} -> {}",
        down.no_stock_ceiling[32] as f64 / Q as f64,
        up.production[32] as f64 / Q as f64
    );

    let cap = capacities(&[(33, 1)]);
    let down = active::collect_demand(&cap);
    let before = down.clone();
    let empty = active::apply_demand(&cap, &[0; R], &down);
    let mut stocks = [0; R];
    stocks[21] = 2 * Q;
    stocks[50] = 3 * Q;
    let up = active::apply_demand(&cap, &stocks, &down);
    assert_eq!(down, before);
    assert_eq!(down.no_stock_ceiling[33], 0);
    assert_eq!(empty.production[33], 0);
    assert_eq!(up.production[33], Q);
    assert_eq!(up.balance[21], -(Q as i64));
    assert_eq!(up.next, 2 * Q);
    println!("PASS stock only in upward: PDA 0 -> 1; Electronics stock drains 1/min, boundary 2 minutes; identical downward plan");
    stocks[21] = 0;
    stocks[50] = Q;
    let depleted = active::apply_demand(&cap, &stocks, &down);
    assert_eq!(depleted.production[33], 0);
    assert_eq!(depleted.next, u64::MAX);
    println!("PASS after the depletion boundary: PDA stops when Electronics stock is empty");

    let cap = capacities(&[(30, 1), (31, 1), (32, 1), (33, 2)]);
    let down = active::collect_demand(&cap);
    let mut stocks = [0; R];
    stocks[21] = 2 * Q;
    stocks[50] = 6 * Q;
    let up = active::apply_demand(&cap, &stocks, &down);
    assert_eq!(up.production[32], Q);
    assert_eq!(up.production[33], 2 * Q);
    assert_eq!(up.balance[21], -(Q as i64));
    assert_eq!(up.next, 2 * Q);
    println!("PASS live production before stock: Electronics demand 2, live 1, stock draw 1/min; boundary 2 minutes");

    let cap = capacities(&[
        (8, 1),
        (23, 1),
        (24, 1),
        (30, 1),
        (31, 1),
        (32, 1),
        (41, 1),
        (42, 1),
        (33, 1),
        (43, 1),
        (48, 3),
        (58, 1),
    ]);
    let down = active::collect_demand(&cap);
    let up = active::apply_demand(&cap, &[0; R], &down);
    let reference = baseline(&cap, &[0; R]);
    assert_eq!(down.relevant[48], Q);
    assert_eq!(down.demand[55], 2 * Q);
    assert_eq!(down.demand[21], 2 * Q);
    println!("PASS first supplier check: Technical Gear requests 1, not 3; total Fibres demand 2, Electronics demand 2");
    println!("\nShared branches: upward changes (resource, recipe, before, after)");
    for &(r, i, b, a) in &up.changes {
        println!(
            "{r} {i} {:.6} {:.6}",
            b as f64 / Q as f64,
            a as f64 / Q as f64
        );
    }
    println!("recipe downward_relevant preliminary_ceiling final baseline");
    for &i in &[33, 43, 48, 58] {
        println!(
            "{i} {} {} {} {}",
            down.relevant[i] as f64 / Q as f64,
            down.no_stock_ceiling[i] as f64 / Q as f64,
            up.production[i] as f64 / Q as f64,
            reference[INPUT + i] as f64 / Q as f64
        );
    }
    assert_eq!(up.production[43], 3 * Q / 8);
    assert_eq!(up.balance[50], Q as i64 / 8);
    assert_eq!(reference[INPUT + 43], Q / 2);
    for &i in &[33, 48, 58] {
        assert_eq!(up.production[i], reference[INPUT + i]);
    }
    assert_eq!(unused(&cap, &[0; R], &up), Some((43, Q / 8)));
    println!("CONFIRMED FAILURE: Composite can consume the spare 0.125 Polymers and Carbon Strands; production can increase from 0.375 to 0.5 without reducing anything else.");

    let mut seed = 91u32;
    let (mut matched, mut different, mut improvable) = (0, 0, 0);
    for test in 0..1000 {
        let mut cap = [0; N];
        for i in 0..N {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            cap[i] = ((seed >> 16) % 5) as u64 * Q;
        }
        let mut stocks = [0; R];
        if test % 2 == 1 {
            for r in 0..R {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                if (seed >> 16) % 3 == 0 {
                    stocks[r] = 10 * Q;
                }
            }
        }
        let down = active::collect_demand(&cap);
        let before = down.clone();
        let up = active::apply_demand(&cap, &stocks, &down);
        assert_eq!(down, before);
        let reference = baseline(&cap, &stocks);
        assert!((0..N).all(|i| up.production[i] <= cap[i]));
        assert!((0..R).all(|r| up.balance[r] >= 0 || stocks[r] > 0));
        if (0..N).all(|i| up.production[i].abs_diff(reference[INPUT + i]) <= 128) {
            matched += 1;
        } else {
            different += 1;
        }
        if unused(&cap, &stocks, &up).is_some() {
            improvable += 1;
        }
    }
    println!("\n1000 generated whole-web cases: baseline_matches={matched}, differences={different}, directly_improvable={improvable}. All capacity/stock feasibility and visit-count checks passed.");
}
