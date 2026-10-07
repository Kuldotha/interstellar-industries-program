# Planet instructions

The default program build exposes the game instructions. The production benchmark entrypoints are only available with the `benchmark` feature and build into `program/target/benchmark`. Never deploy that feature. Both use the shared production, population and fixed-point terrain modules.

## Accounts

| Account | Seeds | Bytes | Lifecycle |
|---|---|---:|---|
| Sponsor | `sponsor`, operator pubkey | 56 | Fund and delegate on base; owns the ER rent budget |
| Planet | `planet`, player pubkey, nonce u64 LE | 6,320 | ER-only; owner, session signer, seed, permutation, clock, tier, buildings, coverage and quest progress |
| Production snapshot | `engine`, planet pubkey | 9,968 | ER-only; shared engine state and stock |
| Topology | `topology`, resolution u32 LE | 23,152 | Shared ER account; sponsor pubkey followed by resolution-8 tile directions and ordered adjacency |

The browser wallet signs a readable authorization message, never an ER transaction. The configured sponsor service verifies the Ed25519 signature, expiration, origin, program, planet, session and one-use request. It co-signs creation/session replacement with the browser session key. This is a trusted-service attestation: the program pins the service authority and validates the sponsoring account; wallet message verification happens in that service. Normal building actions accept the player or the planet’s authorized session signer. Session replacement uses a new wallet-signed message and a transaction signed by the new session and configured service. The direct wallet-authorized instructions remain available for administrative clients. Planet closure requires the owner or authorized session. The login reset flow verifies an explicit wallet-signed deletion message before linking the browser session and closing the colony accounts. Starter creation rejects every nonce except zero, so each wallet has one starter address. Closing a planet refunds the two accounts' ER rent to their original sponsor. A sponsor cannot request undelegation while accounts remain open. Requesting undelegation locks it against new allocations.

The current account layout supports resolution 8 (642 tiles) and the five current building types: quarry 0, concrete factory 1, housing 2, fishing docks 3, Commons 4. The production snapshot retains the full recipe web. Terrain is never accepted from the client: placement evaluates the requested tile using the shared four-octave fixed-point generator. Docks additionally evaluate the ordered neighbor they face. Immutable topology is uploaded from compiled data in three bounded chunks, once per ER, not per planet.

At the SDK's 32 lamports/byte plus 60 bytes per account, a planet and snapshot reserve **525,056 lamports**. Shared topology reserves **742,784 lamports**. Sponsor base rent, delegation costs and transaction fees are additional. Initialization takes an explicit funding amount; it does not choose or spend a default budget.

These planets exist only on the ER. There is no base-layer settlement of planet progress in this free-play implementation.

## Instructions and routing

Discriminators are u64 LE; arguments follow in the listed order. Public keys are 32 raw bytes. The client module builds account metas and payloads.

| Tag | Instruction | Arguments | Route |
|---:|---|---|---|
| 10 | Initialize sponsor | funding u64 | Base |
| 11 | Delegate sponsor | validator pubkey | Base |
| 12 | Prepare topology | none; append next chunk | ER |
| 13 | Close topology | none; sponsor must have no planets | ER |
| 14 | Request sponsor undelegation | none; no open sponsored accounts | ER |
| 15 + callback alias | Process undelegation | SDK seed vector | Base; authenticated delegation callback |
| 16 | Reclaim sponsor | none | Base after undelegation, or before initial delegation |
| 20 | Create planet | nonce u64 (= 0), seed u32, session pubkey | ER |
| 21 | Build | tile u32, kind u8, facing u8 | ER |
| 22 | Demolish | tile u32 | ER |
| 23 | Set paused | tile u32, paused u8 | ER |
| 24 | Advance | none | ER; permissionless, deterministic |
| 25 | Scheduled advance | revision u64 | ER; derived crank signer only |
| 26 | Close planet | none | ER |
| 27 | Authorize session | session pubkey | ER; direct owner signature |
| 28 | Create with session | nonce u64 (= 0), seed u32 | ER; configured service + session signatures, wallet is an unsigned identity |
| 29 | Link session | none | ER; configured service + new session signatures |

`client/onchain.mjs` exports `initializeAndDelegate`, returning **two instructions for one atomic base-layer transaction**. Supply the selected validator identity; discover the sponsor's ER through router `getDelegationStatus` after delegation and wait for propagation. Subsequent ER operations use that endpoint. No live endpoint or validator key is guessed by the client.

Create topology with three `prepareTopology` transactions. Create a planet through the signed-message authorization endpoint; the ER transaction is signed by the session key and configured operator. No scheduler request is made at creation. Start with two concrete and the existing tutorial rewards/unlocks. Demolition refunds the amount originally paid. Worker shortages lower shared efficiency; paused producers do not request workers. Commons coverage updates only the affected radius and counts overlapping utilities once.

## Time and scheduling

Every action uses the Clock sysvar, in whole seconds. It advances the existing rates before checking affordability, applying the action, refreshing rates and updating the schedule. One instruction performs at most one rate solve.

When the next boundary is infinite, there is no task. Steady rates can be projected to the current time. A finite population, depletion or storage-cap boundary requests a task. If its absolute deadline is unchanged, no scheduler CPI is submitted.

The scheduler fires once immediately and then after its interval, so tasks use two iterations. The immediate callback is a no-op before the absolute deadline. Creation seeds its revision from the sponsor’s monotonically increasing creation counter and slot, so closing and recreating an address does not reuse task identity. Each replacement has a fresh task ID and revision; old callbacks do nothing, including after planet closure. Task authority is the planet PDA; callback authority is the crank executor derived for that PDA. Scheduling accepts only the program's own advance instruction.

A late callback advances one boundary and requests the next. A build arriving after an unprocessed boundary returns **Custom(100), catch-up required**, without applying the build. The client should submit `advance` calls until `tick` has caught up, then retry the action. This bounds compute rather than hiding an unbounded catch-up loop in a building transaction. Manual advance also provides recovery if a scheduled attempt fails. Scheduler acceptance is not proof of registration or successful execution.

Other domain errors: 101 occupied tile, 102 locked building, 103 unsuitable terrain/deposit, 104 invalid facing, 105 insufficient concrete, 106 no building, 107 sponsored accounts still open. Standard errors reject invalid owners, signatures, PDAs, account sizes and program identities.

## Build and validation

```sh
node build-onchain.mjs
node client/test.mjs
node build.mjs
npm test
```

The client test expects `@solana/web3.js` or `SOLANA_WEB3_PATH` pointing to an installed copy. It checks instruction serialization and PDA/account-layout parity against the SBF-produced fixture in `onchain-results.json`.

Local tests cover funding/reclaim, authorization, duplicate initialization, progression, placement, shared population, utility overlap, early/stale callbacks, delayed-action rejection, pause, demolition and real CPI serialization. The scheduler test double only records schedule/cancel payloads. It does **not** emulate real scheduler execution or ER account creation/delegation. Never deploy it.

The measured gameplay instruction maximum is recorded in `onchain-results.json`; the full-web engine suite is recorded separately in `population-results.json`. Real Magic Program costs must still be measured on devnet. No live accounts or funds were used for these tests.

Deployed to Solana devnet on 2026-10-03: program `4yysd21qfEAwjt19XiMdYL1GrRytMaBQxUZdiVx1Z5zd`, upgrade authority `7SMQSQ6hhyy1xwPsEoFNWdnGxfR3BYdb4VPkjUrzHU2s`. Persistent keys are in `/Users/tedosijses/keys/interstellar_program.json` and `interstellar_admin.json`. The current binary is 336,552 bytes, verified against the local build. Program allocation is 340,496 bytes: the Solana loader requires extensions of at least 10,240 bytes. See upgrade-devnet.json. See `deployment-devnet.json` for signatures, binary hash and funding accounting. The upload buffer has zero remaining lamports. Remaining funding stays on the admin for sponsor setup.

Sponsor HGx1ouANmqoGvRnLdoL34Byq5KmbFXBReU5g4VQ4Cghw is delegated to devnet-eu (MEUGGrYPxKk17hCr7wpT6s8dtNokZj5U2L57vjYMS8e). Shared topology is initialized. Live tests verify planet creation, duplicate/nonzero-nonce rejection, zero-funded session building, wallet-only session replacement, revocation and planet/engine close with full rent refunds. See er-devnet.json, live-planet-results.json and wallet-api-results.json. Real scheduled callback execution and sponsor undelegation/recovery remain separate validation work.
