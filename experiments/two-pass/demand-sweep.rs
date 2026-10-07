#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DemandSweep {
    pub relevant: [u64; N],
    pub demand: [u64; R],
    pub no_stock_ceiling: [u64; N],
    pub visits: [u8; R],
}

pub struct AppliedSweep {
    pub production: [u64; N],
    pub balance: [i64; R],
    pub next: u64,
    pub visits: [u8; R],
    pub changes: Vec<(usize, usize, u64, u64)>,
}

fn sweep_order() -> Vec<usize> {
    let mut order: Vec<_> = (0..R).collect();
    order.sort_by_key(|&r| *RESOURCE_PRODUCERS[r].iter().max().unwrap());
    for (position, &r) in order.iter().enumerate() {
        for &producer in RESOURCE_PRODUCERS[r] {
            for &input in &RECIPE_RESOURCES[producer][1..] {
                assert!(order[..position].contains(&input));
            }
        }
    }
    order
}

fn supplier_ceiling(cap: &[u64; N], stocks: &[u64; R]) -> [u64; N] {
    let mut metadata = StockMetadata::ZERO;
    metadata.prepare(stocks).unwrap();
    let mut stocked = [false; N];
    for j in 0..RAW {
        stocked[RAW_RECIPES[j]] = stocks[RAW_RESOURCES[j]] > 0;
    }
    let mut result = *cap;
    for v in 0..O {
        let mut possible = OUTPUT_PRODUCERS[v].iter().map(|&i| cap[i]).sum::<u64>();
        for col in metadata.plan().columns[v] {
            let coefficient: i64 = col
                .terms
                .iter()
                .filter(|&&(_, mask)| mask & metadata.stocked == 0)
                .map(|&(coefficient, _)| coefficient)
                .sum();
            if coefficient > 0 {
                if let Some(available) = stock_bound(&metadata.plan().rows[col.row], cap, &stocked)
                {
                    possible = possible.min(available / coefficient as u64);
                }
            }
        }
        for &i in OUTPUT_PRODUCERS[v] {
            result[i] = result[i].min(possible);
        }
    }
    result
}

pub fn collect_demand(cap: &[u64; N]) -> DemandSweep {
    let mut result = DemandSweep {
        relevant: [0; N],
        demand: [0; R],
        no_stock_ceiling: supplier_ceiling(cap, &[0; R]),
        visits: [0; R],
    };
    for &r in sweep_order().iter().rev() {
        result.visits[r] += 1;
        let terminal = !RESOURCE_CONSUMERS[r].iter().any(|&i| cap[i] > 0);
        let wanted = if terminal {
            RESOURCE_PRODUCERS[r].iter().map(|&i| cap[i]).sum()
        } else {
            result.demand[r]
        };
        let allocated = demand_share(
            RESOURCE_PRODUCERS[r],
            cap,
            &result.no_stock_ceiling,
            wanted,
        );
        for &i in RESOURCE_PRODUCERS[r] {
            result.relevant[i] = allocated[i];
            for &input in &RECIPE_RESOURCES[i][1..] {
                result.demand[input] += allocated[i];
            }
        }
    }
    result
}

pub fn apply_demand(cap: &[u64; N], stocks: &[u64; R], down: &DemandSweep) -> AppliedSweep {
    // A stocked intermediate can bypass an empty upstream chain, so its no-stock ceiling is provisional.
    let mut limits = if stocks.iter().any(|&amount| amount > 0) {
        supplier_ceiling(cap, stocks)
    } else {
        down.no_stock_ceiling
    };
    let mut result = AppliedSweep {
        production: [0; N],
        balance: [0; R],
        next: u64::MAX,
        visits: [0; R],
        changes: Vec::new(),
    };
    for r in sweep_order() {
        result.visits[r] += 1;
        for &i in RESOURCE_PRODUCERS[r] {
            result.production[i] = limits[i];
        }
        if stocks[r] > 0 {
            continue;
        }
        let available = RESOURCE_PRODUCERS[r]
            .iter()
            .map(|&i| result.production[i])
            .sum();
        let relevant = std::array::from_fn(|i| down.relevant[i].min(limits[i]));
        let mut shares = demand_share(RESOURCE_CONSUMERS[r], cap, &relevant, available);
        let spent: u64 = RESOURCE_CONSUMERS[r].iter().map(|&i| shares[i]).sum();
        let remaining = std::array::from_fn(|i| limits[i] - shares[i]);
        let spare = demand_share(RESOURCE_CONSUMERS[r], cap, &remaining, available - spent);
        for &i in RESOURCE_CONSUMERS[r] {
            shares[i] += spare[i];
            if shares[i] != limits[i] {
                result.changes.push((r, i, limits[i], shares[i]));
                limits[i] = shares[i];
            }
        }
    }
    for r in 0..R {
        result.balance[r] = RESOURCE_PRODUCERS[r]
            .iter()
            .map(|&i| result.production[i] as i64)
            .sum::<i64>()
            - RESOURCE_CONSUMERS[r]
                .iter()
                .map(|&i| result.production[i] as i64)
                .sum::<i64>();
        if result.balance[r] < 0 {
            assert!(stocks[r] > 0);
            let time = (stocks[r] as u128 * Q as u128 / (-result.balance[r]) as u128) as u64;
            result.next = result.next.min(time);
        }
    }
    assert!(down.visits.iter().all(|&n| n == 1));
    assert!(result.visits.iter().all(|&n| n == 1));
    result
}

fn demand_share(
    consumers: &[usize],
    weights: &[u64; N],
    limits: &[u64; N],
    supply: u64,
) -> [u64; N] {
    let mut result = [0; N];
    if consumers.iter().map(|&i| limits[i]).sum::<u64>() <= supply {
        for &i in consumers {
            result[i] = limits[i];
        }
        return result;
    }
    let mut sorted: Vec<_> = consumers
        .iter()
        .copied()
        .filter(|&i| weights[i] > 0)
        .collect();
    sorted.sort_by(|&a, &b| {
        (limits[a] as u128 * weights[b] as u128).cmp(&(limits[b] as u128 * weights[a] as u128))
    });
    let mut remaining = supply;
    let mut weight: u64 = sorted.iter().map(|&i| weights[i]).sum();
    for (k, &i) in sorted.iter().enumerate() {
        if limits[i] as u128 * weight as u128 <= remaining as u128 * weights[i] as u128 {
            result[i] = limits[i];
            remaining -= limits[i];
            weight -= weights[i];
        } else {
            for &j in &sorted[k..] {
                result[j] = (weights[j] as u128 * remaining as u128 / weight as u128) as u64;
            }
            break;
        }
    }
    result
}
