use super::*;
use std::str::FromStr;
use world::*;
fn key(text: &str) -> Pubkey {
    Pubkey::from_str(text).unwrap()
}
fn instruction(tag: u64, accounts: Vec<AccountMeta>, extra: &[u8]) -> Instruction {
    let mut data = tag.to_le_bytes().to_vec();
    data.extend_from_slice(extra);
    Instruction {
        program_id: key("4yysd21qfEAwjt19XiMdYL1GrRytMaBQxUZdiVx1Z5zd"),
        accounts,
        data,
    }
}
fn bytes<T>(value: &T) -> Vec<u8> {
    unsafe {
        core::slice::from_raw_parts((value as *const T).cast(), core::mem::size_of::<T>()).to_vec()
    }
}
fn account(owner: Pubkey, data: Vec<u8>) -> Account {
    Account {
        lamports: 10_000_000,
        data,
        owner,
        executable: false,
        rent_epoch: 0,
    }
}
fn read_planet(account: &Account) -> &Planet {
    assert_eq!(account.data.as_ptr().align_offset(8), 0);
    unsafe { &*account.data.as_ptr().cast() }
}
#[test]
fn game_instructions_and_schedule_cpi() {
    let program = key("4yysd21qfEAwjt19XiMdYL1GrRytMaBQxUZdiVx1Z5zd");
    let owner = Pubkey::new_from_array([61; 32]);
    let other = Pubkey::new_from_array([62; 32]);
    let sponsor = Pubkey::new_from_array([63; 32]);
    let nonce = 5u64;
    let planet =
        Pubkey::find_program_address(&[b"planet", owner.as_ref(), &nonce.to_le_bytes()], &program)
            .0;
    let engine = Pubkey::find_program_address(&[b"engine", planet.as_ref()], &program).0;
    let top = Pubkey::find_program_address(&[b"topology", &8u32.to_le_bytes()], &program).0;
    let magic = key("Magic11111111111111111111111111111111111111");
    let context = key("MagicContext1111111111111111111111111111111");
    let crank = Pubkey::find_program_address(
        &[b"crank-executor", planet.as_ref()],
        &key("Crank11111111111111111111111111111111111111"),
    )
    .0;
    let mut vm = Mollusk::new(
        &program,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../program/target/deploy/production_lab_cu"
        ),
    );
    vm.compute_budget.compute_unit_limit = 200_000;
    vm.sysvars.clock.unix_timestamp = 100;
    vm.add_program(
        &magic,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../scheduler-test-double/target/deploy/scheduler_test_double"
        ),
        &mollusk_svm::program::loader_keys::LOADER_V3,
    );
    let mut p: Box<Planet> = unsafe {
        Box::from_raw(std::alloc::alloc_zeroed(std::alloc::Layout::new::<Planet>()).cast())
    };
    let mut e = Box::new([0u64; WORDS]);
    p.initialize(&mut e, owner.to_bytes(), sponsor.to_bytes(), nonce, 23, 100);
    let mut t = vec![0; 32];
    t.extend_from_slice(TOPOLOGY);
    let mut accounts = vec![
        (planet, account(program, bytes(&*p))),
        (engine, account(program, bytes(&*e))),
        (top, account(program, t)),
        (owner, Account::default()),
        (other, Account::default()),
        (crank, Account::default()),
        (context, account(magic, vec![0; 528])),
        (
            magic,
            mollusk_svm::program::create_program_account_loader_v3(&magic),
        ),
    ];
    let action_metas = |owner| {
        vec![
            AccountMeta::new_readonly(owner, true),
            AccountMeta::new(planet, false),
            AccountMeta::new(engine, false),
            AccountMeta::new_readonly(top, false),
            AccountMeta::new(context, false),
            AccountMeta::new_readonly(magic, false),
        ]
    };
    let advance_metas = || {
        vec![
            AccountMeta::new(planet, false),
            AccountMeta::new(engine, false),
            AccountMeta::new(context, false),
            AccountMeta::new_readonly(magic, false),
        ]
    };

    let mut max = 0;
    let apply =
        |vm: &Mollusk, accounts: &mut Vec<(Pubkey, Account)>, ix: Instruction, max: &mut u64| {
            let r = vm.process_instruction(&ix, accounts);
            assert!(
                r.raw_result.is_ok(),
                "tag {:?}: {:?} CU {}",
                &ix.data[..8],
                r.raw_result,
                r.compute_units_consumed
            );
            *max = (*max).max(r.compute_units_consumed);
            for (k, a) in r.resulting_accounts {
                if let Some((_, v)) = accounts.iter_mut().find(|(id, _)| id == &k) {
                    *v = a;
                }
            }
        };
    let site = |a: &Vec<(Pubkey, Account)>, kind: u8| {
        let p = world(a, planet);
        for id in 0..TILES {
            if p.tiles[id].kind != 0 {
                continue;
            }
            let own = planet_generation_core::tile(TOPOLOGY, id, &p.permutation, p.seed);
            if own[1] == 0 || (kind == 0 && own[2] & 2 == 0) {
                continue;
            }
            for side in 0..6 {
                let n = neighbor(TOPOLOGY, id, side);
                if n < TILES
                    && (kind != DOCK
                        || planet_generation_core::tile(TOPOLOGY, n, &p.permutation, p.seed)[1]
                            == 0)
                {
                    return (id, side);
                }
            }
        }
        panic!()
    };
    let build_data = |tile: usize, kind: u8, side: usize| {
        let mut d = (tile as u32).to_le_bytes().to_vec();
        d.extend_from_slice(&[kind, side as u8]);
        d
    };
    apply(
        &vm,
        &mut accounts,
        instruction(24, advance_metas(), &[]),
        &mut max,
    );
    assert_eq!(world(&accounts, planet).due, NEVER);
    for _ in 0..2 {
        let (id, side) = site(&accounts, HOUSE);
        let bad = vm.process_instruction(
            &instruction(21, action_metas(other), &build_data(id, HOUSE, side)),
            &accounts,
        );
        assert!(bad.raw_result.is_err());
        apply(
            &vm,
            &mut accounts,
            instruction(21, action_metas(owner), &build_data(id, HOUSE, side)),
            &mut max,
        );
        assert_eq!(world(&accounts, planet).due, NEVER);
    }
    let rotate = |signer, key: Pubkey| instruction(27, vec![AccountMeta::new_readonly(signer, true),AccountMeta::new(planet,false)],key.as_ref());
    assert!(vm.process_instruction(&rotate(other, other), &accounts).raw_result.is_err());
    apply(&vm, &mut accounts, rotate(owner, other), &mut max);
    assert_eq!(world(&accounts, planet).session, other.to_bytes());
    let session_build = vm.process_instruction(&instruction(21, action_metas(other), &build_data(site(&accounts, DOCK).0, DOCK, site(&accounts, DOCK).1)), &accounts);
    assert!(session_build.raw_result.is_ok(), "{:?}", session_build.raw_result);
    assert!(vm.process_instruction(&rotate(other, crank), &accounts).raw_result.is_err());
    apply(&vm, &mut accounts, rotate(owner, crank), &mut max);
    assert!(vm.process_instruction(&instruction(21, action_metas(other), &build_data(site(&accounts, DOCK).0, DOCK, site(&accounts, DOCK).1)), &accounts).raw_result.is_err());
    let (dock, side) = site(&accounts, DOCK);
    apply(
        &vm,
        &mut accounts,
        instruction(21, action_metas(owner), &build_data(dock, DOCK, side)),
        &mut max,
    );
    assert_eq!(world(&accounts, planet).due, 106);
    let generation = world(&accounts, planet).revision;
    let mut metas = vec![AccountMeta::new_readonly(crank, true)];
    metas.extend(advance_metas());
    let callback = instruction(25, metas.clone(), &generation.to_le_bytes());
    let snapshot = accounts.clone();
    apply(&vm, &mut accounts, callback.clone(), &mut max);
    assert_eq!(
        accounts, snapshot,
        "early callback must not change anything"
    );
    let mut wrong = callback.clone();
    wrong.accounts[0].pubkey = other;
    assert!(vm
        .process_instruction(&wrong, &accounts)
        .raw_result
        .is_err());
    vm.sysvars.clock.unix_timestamp = 106;
    apply(&vm, &mut accounts, callback.clone(), &mut max);
    assert_eq!(world(&accounts, planet).tier.population, 6);
    let snapshot = accounts.clone();
    apply(&vm, &mut accounts, callback, &mut max);
    assert_eq!(
        accounts, snapshot,
        "old revision must not repeat or cancel replacement"
    );
    vm.sysvars.clock.unix_timestamp = 130;
    let (id, side) = site(&accounts, HOUSE);
    let stale = vm.process_instruction(
        &instruction(21, action_metas(owner), &build_data(id, HOUSE, side)),
        &accounts,
    );
    assert!(stale.raw_result.is_err());
    for _ in 0..4 {
        apply(
            &vm,
            &mut accounts,
            instruction(24, advance_metas(), &[]),
            &mut max,
        );
    }
    assert_eq!(world(&accounts, planet).tier.population, 10);
    for kind in [0, 1] {
        let (id, side) = site(&accounts, kind);
        apply(
            &vm,
            &mut accounts,
            instruction(21, action_metas(owner), &build_data(id, kind, side)),
            &mut max,
        );
    }
    assert_eq!(world(&accounts, planet).clock.workers, 10);
    assert_eq!(world(&accounts, planet).phase, 3);
    let mut pause = (dock as u32).to_le_bytes().to_vec();
    pause.push(1);
    apply(
        &vm,
        &mut accounts,
        instruction(23, action_metas(owner), &pause),
        &mut max,
    );
    assert_eq!(world(&accounts, planet).clock.workers, 5);
    apply(
        &vm,
        &mut accounts,
        instruction(22, action_metas(owner), &(dock as u32).to_le_bytes()),
        &mut max,
    );
    assert_eq!(world(&accounts, planet).tiles[dock].kind, 0);
    let data = &accounts.iter().find(|(k, _)| *k == planet).unwrap().1.data;
    let hex = data.iter().map(|b| format!("{b:02x}")).collect::<String>();
    std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"),"/../onchain-results.json"),format!("{{\"maxCuWithSchedulerDouble\":{max},\"planetBytes\":{},\"engineBytes\":{},\"owner\":\"{owner}\",\"planet\":\"{planet}\",\"engine\":\"{engine}\",\"topology\":\"{top}\",\"nonce\":5,\"planetData\":\"{hex}\"}}",core::mem::size_of::<Planet>(),WORDS*8)).unwrap();
    {
      let data=&mut accounts.iter_mut().find(|(k,_)|*k==planet).unwrap().1.data;
      let p=unsafe{&mut *data.as_mut_ptr().cast::<Planet>()};p.peak=100;
      let data=&mut accounts.iter_mut().find(|(k,_)|*k==engine).unwrap().1.data;
      let e=unsafe{&mut *data.as_mut_ptr().cast::<[u64;WORDS]>()};e[stock_index(CONCRETE)]=100*Q;
    }
    for kind in 5..=11 {
      let (tile,side)=site(&accounts,kind);let mut extra=(tile as u32).to_le_bytes().to_vec();extra.extend_from_slice(&[kind,side as u8]);
      apply(&vm,&mut accounts,instruction(21,action_metas(owner),&extra),&mut max);
    }

    {
      let data=&mut accounts.iter_mut().find(|(k,_)|*k==planet).unwrap().1.data;data[..8].copy_from_slice(b"IIWORLD1");
      let data=&mut accounts.iter_mut().find(|(k,_)|*k==engine).unwrap().1.data;let e=unsafe{&mut *data.as_mut_ptr().cast::<[u64;WORDS]>()};e[8]=Q;e[10]=Q;e[12]=Q;
    }
    let before=world(&accounts,planet).tiles;
    let mut migration=vec![AccountMeta::new_readonly(owner,true)];migration.extend(advance_metas());
    apply(&vm,&mut accounts,instruction(31,migration,&[]),&mut max);
    assert_eq!(world(&accounts,planet).tag,PLANET_TAG);assert_eq!(bytes(&before),bytes(&world(&accounts,planet).tiles));
    for(farm_kind,field_kind)in [(5,12),(7,13),(9,14),(10,15)]{
      let p=world(&accounts,planet);let farm=p.tiles.iter().position(|t|t.kind==farm_kind+1).unwrap();
      let fields:Vec<_>=(0..6).map(|side|neighbor(TOPOLOGY,farm,side)).filter(|&n|n<TILES&&p.tiles[n].kind==0&&planet_generation_core::tile(TOPOLOGY,n,&p.permutation,p.seed)[1]>0).take(3).collect();
      assert!(!fields.is_empty());
      let workforce=p.clock.workers;let expected_generators=1+fields.len() as u64;
      for field in fields{let side=(0..6).find(|&side|neighbor(TOPOLOGY,field,side)==farm).unwrap();let mut extra=(field as u32).to_le_bytes().to_vec();extra.extend_from_slice(&[field_kind,side as u8]);apply(&vm,&mut accounts,instruction(21,action_metas(owner),&extra),&mut max);}
      assert_eq!(world(&accounts,planet).clock.workers,workforce+if farm_kind==10{(expected_generators-1)*5}else{0});
      if farm_kind==10{assert_eq!(world(&accounts,planet).needs.generators,expected_generators);}
      apply(&vm,&mut accounts,instruction(22,action_metas(owner),&(farm as u32).to_le_bytes()),&mut max);
      assert_eq!(world(&accounts,planet).fields(farm),0);
    }
    apply(&vm,&mut accounts,instruction(24,advance_metas(),&[]),&mut max);
    println!("Game SBF instruction maximum with scheduler CPI test double: {max} CU");
}

fn world(a: &[(Pubkey, Account)], planet: Pubkey) -> &Planet {
    read_planet(&a.iter().find(|(k, _)| *k == planet).unwrap().1)
}
#[test]
fn sponsor_funding_authority_and_reclaim() {
    let program = key("4yysd21qfEAwjt19XiMdYL1GrRytMaBQxUZdiVx1Z5zd");
    let admin = Pubkey::new_from_array([81; 32]);
    let stranger = Pubkey::new_from_array([82; 32]);
    let sponsor = Pubkey::find_program_address(&[b"sponsor", admin.as_ref()], &program).0;
    let vm = Mollusk::new(
        &program,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../program/target/deploy/production_lab_cu"
        ),
    );
    let system = mollusk_svm::program::keyed_account_for_system_program();
    let funds = 700_000u64;
    let accounts = vec![
        (
            admin,
            Account {
                lamports: 5_000_000,
                ..Account::default()
            },
        ),
        (sponsor, Account::default()),
        (stranger, Account::default()),
        system.clone(),
    ];
    let ix = instruction(
        10,
        vec![
            AccountMeta::new(admin, true),
            AccountMeta::new(sponsor, false),
            AccountMeta::new_readonly(system.0, false),
        ],
        &funds.to_le_bytes(),
    );
    let r = vm.process_instruction(&ix, &accounts);
    assert!(r.raw_result.is_ok(), "{:?}", r.raw_result);
    let made = &r
        .resulting_accounts
        .iter()
        .find(|(k, _)| *k == sponsor)
        .unwrap()
        .1;
    assert_eq!(made.owner, program);
    assert_eq!(made.data.len(), 56);
    assert_eq!(made.lamports, vm.sysvars.rent.minimum_balance(56) + funds);
    assert!(vm
        .process_instruction(&ix, &r.resulting_accounts)
        .raw_result
        .is_err());
    let bad = instruction(
        16,
        vec![
            AccountMeta::new(stranger, true),
            AccountMeta::new(sponsor, false),
        ],
        &[],
    );
    let mut after = r.resulting_accounts;
    after.push((stranger, Account::default()));
    assert!(vm.process_instruction(&bad, &after).raw_result.is_err());
    let closed = vm.process_instruction(
        &instruction(
            16,
            vec![
                AccountMeta::new(admin, true),
                AccountMeta::new(sponsor, false),
            ],
            &[],
        ),
        &after,
    );
    assert!(closed.raw_result.is_ok(), "{:?}", closed.raw_result);
    assert_eq!(
        closed
            .resulting_accounts
            .iter()
            .find(|(k, _)| *k == sponsor)
            .unwrap()
            .1
            .lamports,
        0
    );
    assert_eq!(
        closed
            .resulting_accounts
            .iter()
            .find(|(k, _)| *k == admin)
            .unwrap()
            .1
            .lamports,
        5_000_000
    );
}

#[test]
fn message_session_links_require_the_configured_authority() {
    let program = key("4yysd21qfEAwjt19XiMdYL1GrRytMaBQxUZdiVx1Z5zd");
    let attacker = Pubkey::new_from_array([89; 32]);
    let session = Pubkey::new_from_array([90; 32]);
    let planet = Pubkey::new_from_array([91; 32]);
    let sponsor = Pubkey::new_from_array([92; 32]);
    let vm = Mollusk::new(&program, concat!(env!("CARGO_MANIFEST_DIR"), "/../program/target/deploy/production_lab_cu"));
    let accounts = [attacker, session, planet, sponsor].map(|k| (k, Account::default()));
    let ix = instruction(29, vec![
        AccountMeta::new_readonly(attacker, true),
        AccountMeta::new_readonly(session, true),
        AccountMeta::new(sponsor, false),
        AccountMeta::new(planet, false),
    ], &[]);
    let result = vm.process_instruction(&ix, &accounts);
    assert!(matches!(result.raw_result, Err(solana_instruction::error::InstructionError::MissingRequiredSignature)));
}
