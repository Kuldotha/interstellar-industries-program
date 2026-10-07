mod demand_checks;

include!("../../common.rs");
use mollusk_svm::Mollusk;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
fn run(
    vm: &mut Mollusk,
    t: &[u8],
    state: &[u8],
    mode: u8,
    start: usize,
    count: usize,
    seed: u32,
) -> (Option<Vec<u8>>, u64) {
    let program = Pubkey::new_from_array([51; 32]);
    let topo = Pubkey::new_from_array([52; 32]);
    let planet = Pubkey::new_from_array([53; 32]);
    let acc = |data: Vec<u8>| Account {
        lamports: 1_000_000_000,
        data,
        owner: program,
        executable: false,
        rent_epoch: 0,
    };
    let mut data = vec![mode];
    data.extend((start as u32).to_le_bytes());
    data.extend((count as u32).to_le_bytes());
    data.extend(seed.to_le_bytes());
    let ix = Instruction {
        program_id: program,
        accounts: vec![
            AccountMeta::new_readonly(topo, false),
            AccountMeta::new(planet, false),
        ],
        data,
    };
    let result = vm.process_instruction(
        &ix,
        &[(topo, acc(t.to_vec())), (planet, acc(state.to_vec()))],
    );
    if result.raw_result.is_err() {
        return (None, result.compute_units_consumed);
    }
    let out = result
        .resulting_accounts
        .into_iter()
        .find(|(key, _)| *key == planet)
        .unwrap()
        .1
        .data;
    (Some(out), result.compute_units_consumed)
}
fn main() {
    let root = std::env::args().nth(1).unwrap();
    let program = Pubkey::new_from_array([51; 32]);
    let mut vm = Mollusk::new(
        &program,
        &format!("{root}/program/target/deploy/planet_generation_cu"),
    );
    demand_checks::check(&mut vm, &root);
    for resolution in [5, 8, 12, 16] {
        let t = std::fs::read(format!("{root}/fixtures/topology-{resolution}.bin")).unwrap();
        let expected = std::fs::read(format!("{root}/fixtures/expected-{resolution}.bin")).unwrap();
        let n = tiles(&t);
        let blank = vec![0; state_size(n)];

        vm.compute_budget.compute_unit_limit = 200_000;
        for seed in [1701u32, 42, 0, u32::MAX] {
            assert!(run(&mut vm, &t, &blank, 6, 0, 1, seed).0.is_none());
            let (initialized, setup_cu) = run(&mut vm, &t, &blank, 5, 0, 0, seed);
            let initialized = initialized.unwrap();
            let mut native = blank.clone();
            initialize_permutation(&mut native, n, seed);
            assert_eq!(initialized, native);
            assert!(run(&mut vm, &t, &initialized, 5, 0, 0, seed).0.is_none());
            assert!(run(&mut vm, &t, &initialized, 6, 0, 1, seed ^ 1)
                .0
                .is_none());
            terrain(&t, &mut native, 0, n, 4, seed);
            for chunk in [12, 16] {
                let mut state = initialized.clone();
                let (mut sum, mut max, mut calls) = (0u64, 0u64, 0usize);
                for start in (0..n).step_by(chunk) {
                    let (out, cu) = run(&mut vm, &t, &state, 6, start, chunk.min(n - start), seed);
                    state = out.expect("cached generation exceeded budget");
                    sum += cu;
                    max = max.max(cu);
                    calls += 1;
                }
                assert_eq!(state, native);
                println!(
                    "{}",
                    format!(
                        r#"{{"resolution":{resolution},"mode":6,"seed":{seed},"chunk":{chunk},"setupCu":{setup_cu},"generationCu":{sum},"totalCu":{},"maxCu":{max},"calls":{calls},"complete":true}}"#,
                        sum + setup_cu
                    )
                );
            }
        }
        for mode in [0, 1, 2, 4] {
            let mut cpu = blank.clone();
            terrain(&t, &mut cpu, 0, n, mode, 1701);
            if mode == 4 {
                let mut float = blank.clone();
                terrain(&t, &mut float, 0, n, 1, 1701);
                let differences = (0..n)
                    .filter(|&i| cpu[8 + i * 4] != float[8 + i * 4])
                    .count();
                println!(
                    "{{\"resolution\":{resolution},\"fixedHeightDifferences\":{differences}}}"
                );
            }
            if mode == 0 {
                for i in 0..n {
                    assert_eq!(
                        &cpu[8 + i * 4..11 + i * 4],
                        &expected[i * 4..i * 4 + 3]
                            .iter()
                            .enumerate()
                            .map(|(j, &v)| if j == 2 { v & 2 } else { v })
                            .collect::<Vec<_>>()[..],
                        "terrain tile {i}"
                    );
                }
            }
            for budget in [200_000, 1_400_000] {
                vm.compute_budget.compute_unit_limit = budget;
                let (out, cu) = run(&mut vm, &t, &blank, mode, 0, n, 1701);
                if let Some(ref out) = out {
                    assert_eq!(out, &cpu);
                }
                println!("{{\"resolution\":{resolution},\"tiles\":{n},\"mode\":{mode},\"budget\":{budget},\"cu\":{cu},\"complete\":{}}}",out.is_some());
            }
            vm.compute_budget.compute_unit_limit = 1_400_000;
            let (single, cu) = run(&mut vm, &t, &blank, mode, 0, 1, 1701);
            assert!(single.is_some());
            println!("{{\"resolution\":{resolution},\"mode\":{mode},\"sampleTileCu\":{cu}}}");
            vm.compute_budget.compute_unit_limit = 200_000;
            for chunk in [1, 2, 12, 16, 24, 32] {
                let mut state = blank.clone();
                let (mut sum, mut max, mut done, mut calls) = (0, 0, true, 0);
                for start in (0..n).step_by(chunk) {
                    let (out, cu) =
                        run(&mut vm, &t, &state, mode, start, chunk.min(n - start), 1701);
                    let Some(out) = out else {
                        done = false;
                        break;
                    };
                    state = out;
                    sum += cu;
                    max = max.max(cu);
                    calls += 1;
                }
                if done {
                    assert_eq!(state, cpu);
                }
                println!("{{\"resolution\":{resolution},\"mode\":{mode},\"chunk\":{chunk},\"totalCu\":{sum},\"maxCu\":{max},\"calls\":{calls},\"complete\":{done}}}");
            }
            if mode == 0 {
                let mut reference = cpu.clone();
                forests(&t, &mut reference, 1701);
                for i in 0..n {
                    assert_eq!(
                        reference[8 + i * 4 + 2],
                        expected[i * 4 + 2],
                        "forest tile {i}"
                    );
                }
                for budget in [200_000, 1_400_000] {
                    vm.compute_budget.compute_unit_limit = budget;
                    let (out, cu) = run(&mut vm, &t, &cpu, 3, 0, n, 1701);
                    if let Some(out) = &out {
                        assert_eq!(out, &reference);
                    }
                    println!("{{\"resolution\":{resolution},\"tiles\":{n},\"mode\":3,\"budget\":{budget},\"cu\":{cu},\"complete\":{}}}",out.is_some());
                }
            }
        }
    }
}
