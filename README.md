# Interstellar Industries — Program

On-chain program, shared Rust runtime, planet generation crates, and production test harness. The game and model editor are in the sibling `../interstellar-industries-client` project.

## Local development

Use Node 24 (`nvm use`), then `npm run dev`. The production test harness runs on port 5174. `npm run engine` builds WASM, the SBF benchmark, and the local CU runner. Build artifacts are stored in `target/` and the individual program target directories.

## Production lab

Run from this directory:

```sh
npm install
npm run engine
npm run dev
```

Open http://127.0.0.1:5174/. Requires Rust with `wasm32-unknown-unknown`, the Solana SBF builder, and the cached Rust dependencies used by the simulation benchmarks. `build.mjs` builds Wasm, SBF and the local Mollusk runner; generated unfunded program keypairs are removed in a `finally` block. No on-chain accounts or funds are used.

`npm test` compares complete Wasm and SBF state bit-for-bit for construction sequences, the 205 fixed-point regression fixtures, and reserve depletion. `stock-test.mjs` checks intermediate stock, alternative producers, mixed inventories, incremental construction and workforce changes. `npm run build` bundles frontend assets; real CU measurement requires the Vite development server's local emulator endpoint and is not available from static hosting alone.

## Program interface

The SBF wrapper uses Solarium's Pinocchio backend, pinned to revision `85f77a6ac12ea26a65b5e950a0f463e898399939`. The shared simulation kernel is also compiled into Wasm. Solarium's generated dispatcher uses std; the simulation operates directly on the borrowed account data without serializing or copying the planet state.

`ProductionLab::update` takes one writable, program-owned account and a u8 action. Its instruction bytes are the little-endian u64 discriminator 0 followed by the action byte. Actions 0–83 build a recipe group, 254 updates workforce, and 255 recalculates. Account size is 9,968 bytes. The local runner uses this same wire format. This remains a simulation test harness, not a deployed or player-authorized game program.

## Controls

Search/filter the 84 recipes and add buildings individually. Each unpaused building adds one output unit per minute of capacity. Health and power are fixed at 100%. Workforce defaults to full staffing; the available-workers control allows shortage tests. Each building requests one worker in this test. Effective capacity uses the shared multiplier min(1, available/requested). Buildings are aggregated into 84 recipe capacities. The production panel shows actual throughput, efficiency and net resource flow.

Raw and manufactured stocks can be set to test production above live supply. For example, one Concrete Fabricator and 10 Granules, without a Crusher, produces one Concrete Block per minute and predicts depletion after 10 minutes. Adding a Crusher eliminates that depletion boundary.

This is a snapshot/rate test. The clock does not advance. Raw and manufactured stocks cover production deficits. Per-resource storage caps limit net accumulation while preserving production consumed immediately. For each resource the engine records stock drawdown as max(consumption − live production, 0), then takes the earliest stock-depletion or storage-fill time.

## Measurements

Every build runs the shared Rust function in Wasm, then replays the same pre-action state in a local Mollusk SBF execution with a hard 200,000 CU budget. The browser verifies every output word against Wasm, including group capacities, rates, balances, next boundary and solver stage count. Failed or mismatched actions revert the browser state.

- Metadata CU covers validation, group capacity, workforce calculation and dirty-component selection. Recipe graph metadata is fixed and retained, so construction does not regenerate it.
- Solver CU covers rate allocation, alternative-route recovery, balances and next-depletion time.
- Total CU additionally includes instruction handling and instrumentation. Scheduler and transaction-wrapper costs are excluded.
- Reset and reserve edits are test setup operations. Their log explicitly says their mutations are unmetered; only recalculation is measured for them.
- Browser milliseconds are separate from CU. No runtime-to-CU conversion is used.

The local endpoint validates bounded fixed-point input. All state is temporary and reload starts an empty planet. No wallet is required.

## Incremental updates

The account retains the previous solution and workforce multiplier. A build with unchanged workforce directly updates a raw source when its previous capacity already covers all consumer capacity, or positive stock makes its supply constraint inactive. The source balance changes by the production delta, preserving existing consumption. Otherwise a bounded queue walks only built recipe groups sharing input/output resources, including competing producers and consumers. Unbuilt factories do not connect active chains. Only reached groups are re-solved; other rates remain cached. If the multiplier changes, components are marked dirty only when their effective Q24 capacities change. Independent components retain their previous rates and balances. The global minimum stock depletion time is refreshed after updates.

Fully productive, stock-free components can be updated to their new capacities without solving, provided all new resource balances remain feasible. Components constrained by inputs or supported by stock are re-solved. This is deliberately narrower than arbitrary floating-point uniform scaling: Q24 rounding can change allocations, so constrained solutions are not scaled approximately. The workforce-rescaling certificate still uses conservative static components. The subsequent solve uses active dependencies; generated projection rows without enabled outputs and alternative routes without capacity are skipped.

The resource-demand panel shows requested input at current workforce capacity, actual live supply and actual consumption. Requested demand includes factories blocked by other resources; it is not a claim that every highlighted input is the current binding bottleneck. Find producers filters the catalogue to buildings producing that resource.

`npm test` now checks incremental results against a fresh full solve in addition to Wasm/SBF equality. It covers construction in multiple orders, workforce changes including zero, and stock support. The ledger stores per-group execution flags (solved, direct update, rescaled), so it exposes what was recalculated. The browser measurement endpoint replays the entire cached account state, rather than recomputing an unmetered prior solution. This endpoint is a local test harness, not an authoritative game service.

The regression suite compares incremental updates with full solves and verifies Wasm/SBF equality under a hard 200,000 CU budget. Measurements exclude scheduler overhead and are not a worst-case bound.

## Stock demand metadata

The Rust build tool prepares conditional demand coefficients from the recipe graph. A stocked intermediate stops downstream demand expanding into its upstream ingredients; its actual production is still allocated from available capacity and inputs. Coefficients are cached in account memory and refreshed only when intermediate stock changes between empty and positive. Worker and building-count changes reuse them. Both stocked and stock-free allocation use the same progressive allocation routine in active_kernel.rs; no runtime constraint elimination is performed. The existing full-solve reference remains for regression comparison.

The complete cache is included in Wasm/SBF state replay and CU measurements. Stock amount edits are test setup; conditional metadata preparation, allocation and depletion calculation are metered. Stock is not consumed by this snapshot test: the displayed boundary reports when recalculation would next be necessary.

## CU profiling

Run `npm run profile` to replay both regression suites, select their highest-cost cases, and compare normal SBF execution with an instrumented build. Results are saved in `profile-results.json`. The instrumented binary is temporary; the browser keeps using the normal binary. Both execute under 200,000 CU, and their complete output states must match.

Section timings include their timing-read cost. The first ten sections plus unattributed work equal the instrumented total. Allocation selection, advancement and retirement are nested details and must not be added again. Normal totals and instrumentation overhead are recorded separately. Measurements exclude scheduler overhead.

## Storage caps

The shared Rust allocator enforces zero positive net production for resources whose storage is full. Production can still replace consumer demand, and an underpowered producer can keep running while stock falls. Unpaused buildings retain their workforce demand. Capped manufactured outputs participate in the same bounded allocation stages, including alternative routes; raw sources are limited to the resulting consumption because they consume no ingredients. Full-store conditions are included in cached stock metadata.

The account appends 80 capacity words at the WASM `caps_offset()` word offset. Zero disables a limit; a finite cap is encoded as its Q24 value plus one, so a zero-sized buffer is representable. Set stock and caps before recalculation. The build exports the offset in `public/engine-layout.json`. Existing account data must be migrated to the new layout before use.

The next boundary is the minimum of stock divided by net drawdown and remaining storage divided by positive net production. This remains a rate/snapshot harness: it does not advance time or schedule a crank. External raw-resource consumption reserves live supply first and draws any deficit from stock; without stock, fulfillment is limited to available supply. Manufactured external demand is rejected. The game runtime uses this interface for housing fish consumption.

`npm run test:caps` exercises full-store replenishment, zero buffers, alternative producers, freed inputs, all-full storage, mixed stocks and caps, construction, and workforce changes. It checks original-recipe balances, full-store nonoverflow, locally feasible unused production, full/incremental agreement, and exact WASM/SBF state equality under a 200,000 CU instruction limit. Results are in `caps-results.json`; scheduler overhead is excluded.

## Shared population runtime

`population.rs` supplies housing fulfillment, growth/decline, workforce normalization and exact stock projection to both the browser game and the SBF program. Each population tier stores five words: house count, total population, Commons-covered house count, food fulfillment (Q24), and a shared growth phase. Coverage is a union count; overlapping utilities do not duplicate bonuses. The population target is `2 × houses + 5 × covered houses + floor(3 × houses × food fulfillment)`. Food fulfillment is supplied consumption divided by desired consumption; stock covers deficits until depletion. Population changes together at six-second growth and two-second decline boundaries. The snapshot harness accepts up to eight tiers, with total population and worker demand limited to 65,536. The game currently has one colonist tier.

`colony.rs` advances a snapshot to the earlier of the requested time or its next boundary. Need fulfillment, resident changes, stock exhaustion and storage fill determine the boundary. A stable colony can skip elapsed time. A late caller repeats bounded advances until caught up; a single instruction performs at most one production solve. Initialization or externally changed capacities require a refresh before advancement. Duplicate timestamps do not repeat economic effects.

The SBF `advance` instruction (discriminator 1, followed by an initialization byte) reads the Clock sysvar and updates the production snapshot, clock and tier records directly in account memory. Its account size is `(WORDS + CLOCK_WORDS + 5 * tier_count) * 8`; there is no fixed allocation for unused housing slots. The account stores the next absolute update time. Scheduling CPI, deployment, account initialization/authorization and game placement instructions are not part of this simulation harness.

`npm run test:population` checks exact WASM/SBF snapshots through growth, decline, shared workforce, full-web stocks and caps, duplicate execution and stable advancement. `population-results.json` records instruction costs with a 200,000 CU limit. These costs include advancement, rate refresh, Clock access and account writes, but exclude scheduler execution. The browser game parity test is `node ../interstellar-industries-client/tests/population-parity.mjs`.

## On-chain game instructions

The default program now exposes authorized planet and building instructions, sponsor funding/delegation, and boundary scheduling. See [ONCHAIN.md](ONCHAIN.md) for account layouts, transaction routing, client builders and validation limits. The benchmark entrypoints require the explicit `benchmark` feature and are excluded from the default program.

## Licence

Copyright (c) 2026 Kuldotha. All rights reserved.

Source published for review. No licence to reuse, modify, redistribute, or commercially exploit this software or its associated original assets is granted, except as permitted by applicable law or the repository hosting platform’s terms. Written permission is required for other uses. See [LICENSE](LICENSE).

Third-party dependencies and assets retain their respective licences.
