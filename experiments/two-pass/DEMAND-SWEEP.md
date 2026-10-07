# Demand collection and upward application

Run from the workspace root:

```sh
node /Users/tedosijses/Projects/interstellar-industries-program/experiments/two-pass/demand-run.mjs
```

This is a Rust experiment using the engine's 84 recipes, 80 resources and expanded supplier constraints. The runner copies engine files into a temporary directory and removes the directory in `finally`. It does not replace the working solver or browser engine.

## Downward collection

`collect_demand(cap)` has no stock parameter. It records independent no-stock supplier ceilings, then traverses resources from consumers towards suppliers. Active terminal products request their capacity capped by those ceilings; intermediate suppliers receive their consumers' aggregated requests, also capped by those ceilings. It records relevant production and input demand without allocating actual production.

Technical Gear capacity 3 with a no-stock supplier ceiling of 1 requests only 1. Together with Furniture and PDAs, this yields total Rough Fibres demand 2 and Electronics demand 2. Regression assertions verify these values. Installed capacity remains available separately so stock can lift a preliminary ceiling during upward application.

## Upward application

`apply_demand(cap, stocks, down)` establishes stock-aware independent supplier ceilings if any stock exists. This is a bounded scan of the existing expanded constraints, not a call to the production allocator. A stocked intermediate bypasses upstream requirements when deriving these ceilings.

The function traverses resources from suppliers towards consumers. It calculates live production, shares it among consumers up to their relevant demand, and offers any remainder towards additional production. Shares are weighted by recipe-group capacity and capped by each consumer's current limits. The upward pass lowers provisional limits as shared constraints are encountered; it does not freeze the downward estimates.

Positive stock removes the immediate supply limit for that resource. After rates are established, net consumption minus production determines stock drain. The minimum stock/drain ratio gives the depletion boundary.

Each resource is visited once in each graph sweep. Capacity-bound preparation and final balance accounting are separate bounded scans. Sharing uses sorting and a bounded prefix scan, with no iterative convergence or repeated global constraint selection.

## Verified controls

- Known local bottleneck: Electronics 1 and Superconductors 9 share Copper 10 correctly.
- A preliminary Electronics ceiling of 1 is reduced in the upward pass when Copper is shared.
- Stocked Electronics and Polymers allow PDA production despite zero live suppliers and a zero no-stock ceiling. The downward plan remains identical.
- Live Electronics production 1 plus stock supports PDA demand 2; Electronics stock drains at 1, not 2.
- The calculated depletion boundary is 2 minutes in both stock controls. Recalculation with depleted Electronics stops the stock-only PDA factory.

## Confirmed failure in upward allocation order

All supplier capacities below are 1. PDA, Composite and Furniture capacities are 1; Technical Gear capacity is 3. Each product consumes one of each listed ingredient:

| Product | Inputs |
|---|---|
| PDA | Electronics, Polymers |
| Composite | Carbon Strands, Polymers |
| Technical Gear | Electronics, Rough Fibres |
| Furniture | Rough Fibres, Polymers |

All independent preliminary product ceilings are 1. Stock is zero.

1. Rough Fibres: Technical Gear receives 0.75, Furniture 0.25.
2. Polymers: Furniture receives 0.25; PDA and Composite receive 0.375 each.
3. Electronics: Technical Gear receives 0.75, so PDA falls to 0.25.

The last step frees 0.125 Polymers after their allocation has been visited. Composite stays at 0.375. The test proves it can increase to 0.5 using available Polymers and Carbon Strands without decreasing any other output. The existing solver returns that improved result.

This isolates a failure of this fixed upward allocation order. It does not prove that all algorithms with downward collection and upward application must fail. It shows that independent supplier ceilings plus aggregated downward requests do not yet resolve the competing shared limits before a resource's single upward allocation.

## Broader diagnostic

1,000 deterministic whole-web cases alternate empty stock and mixed stock. All capacity, nonnegative-unstocked-balance and visit-count assertions pass. 102 match baseline rates within 128 Q24 ticks; 898 differ; 539 have a factory that can directly increase with available inputs. Baseline differences alone are not proofs of wasted input: this local relevant-first allocation is not established equivalent to the baseline's global final-output priorities or alternative-route allocation. The explicit shared-branch test avoids those ambiguities.

The candidate is incorrect and remains isolated. SBF CU has not been measured for it; native execution time is not a CU estimate.
