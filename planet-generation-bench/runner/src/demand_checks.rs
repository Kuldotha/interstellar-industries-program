use super::*;
pub fn check(vm: &mut Mollusk, root: &str) {
    let mut maps = Vec::new();
    for resolution in [5, 8, 12, 16] {
        let t = std::fs::read(format!("{root}/fixtures/fixed-topology-{resolution}.bin")).unwrap();
        let n = tiles(&t);
        for seed in [0u32, 42, 1701, u32::MAX] {
            vm.compute_budget.compute_unit_limit = 200_000;
            let blank = vec![0; on_demand::STATE_SIZE];
            assert!(run(vm, &t, &blank, 7, 0, 0, seed).0.is_none());
            let (init, setup) = run(vm, &t, &blank, 9, 0, 0, seed);
            let init = init.unwrap();
            let mut native = blank.clone();
            on_demand::initialize(&mut native, seed);
            assert_eq!(init, native);
            assert!(run(vm, &t, &init, 9, 0, 0, seed).0.is_none());
            assert!(run(vm, &t, &init, 7, n, 0, seed).0.is_none());
            assert!(run(vm, &t, &init, 8, 0, 6, seed).0.is_none());
            assert!(run(vm, &t, &init, 7, 0, 0, seed ^ 1).0.is_none());
            let mut records = Vec::new();
            let (mut ordinary, mut docks, mut checks) = (0, 0, 0);
            for id in 0..n {
                on_demand::query(&t, &mut native, id, None);
                records.push(native[264..267].to_vec());
                if resolution == 8 || id % 17 == 0 {
                    let (out, cu) = run(vm, &t, &init, 7, id, 0, seed);
                    assert_eq!(out.unwrap(), native);
                    ordinary = ordinary.max(cu);
                    checks += 1;
                    for side in 0..6 {
                        let mut expected = init.clone();
                        on_demand::query(&t, &mut expected, id, Some(side));
                        let (out, cu) = run(vm, &t, &init, 8, id, side, seed);
                        assert_eq!(out.unwrap(), expected);
                        docks = docks.max(cu);
                        checks += 1;
                    }
                }
            }
            for id in (0..n).rev() {
                on_demand::query(&t, &mut native, id, None);
                assert_eq!(&native[264..267], records[id]);
            }
            for id in 0..n {
                let first = adjacent(&t, id, 0);
                for side in 0..6 {
                    let next = adjacent(&t, id, side);
                    if next < n {
                        assert!(first <= next);
                        assert!((0..6).any(|s| adjacent(&t, next, s) == id));
                    }
                    on_demand::query(&t, &mut native, id, Some(side));
                    assert_eq!(
                        native[268],
                        u8::from(records[id][1] != 0 && next < n && records[next][1] == 0)
                    );
                }
            }
            let forest = records.iter().filter(|r| r[2] == 1).count();
            let land = records.iter().filter(|r| r[1] == 1).count();
            let mut isolated = 0;
            for id in 0..n {
                if records[id][2] == 1
                    && (0..6).all(|s| {
                        let j = adjacent(&t, id, s);
                        j >= n || records[j][2] != 1
                    })
                {
                    isolated += 1;
                }
            }
            println!("{{\"resolution\":{resolution},\"seed\":{seed},\"onDemand\":true,\"setupCu\":{setup},\"tileMaxCu\":{ordinary},\"dockMaxCu\":{docks},\"sbfChecks\":{checks},\"greenTiles\":{land},\"forestTiles\":{forest},\"isolatedForestTiles\":{isolated}}}");
            let coords: Vec<_> = (0..n)
                .map(|i| {
                    let d: Vec<_> = (0..3)
                        .map(|j| {
                            i32::from_le_bytes(
                                t[8 + i * 36 + j * 4..12 + i * 36 + j * 4]
                                    .try_into()
                                    .unwrap(),
                            )
                        })
                        .collect();
                    d
                })
                .collect();
            maps.push(format!("{{\"resolution\":{resolution},\"seed\":{seed},\"coords\":{coords:?},\"tiles\":{records:?}}}"));
        }
    }
    std::fs::write(format!("{root}/maps.json"), format!("[{}]", maps.join(","))).unwrap();
}
