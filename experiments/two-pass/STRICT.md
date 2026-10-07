# Fixed graph sweeps

Run `node /Users/tedosijses/Projects/interstellar-industries-program/experiments/two-pass/strict-run.mjs` from the workspace root. Temporary build files are removed in `finally`. Production engine files are copied, not modified.

The experiment first computes independent capacity ceilings using the existing expanded supplier constraints. It then visits resources in reverse topological order, recording consumer MRE bounds, followed by forward topological application with immutable MREs. Each resource is visited exactly once in each sweep. Scarce resources use a sorted, bounded proportional allocation. There is no repeated global constraint selection.

The local bottleneck example passes: Electronics is limited to 1 before sharing 10 Copper with Superconductors, which receive 9.

The shared-branch example fails the no-avoidable-surplus requirement:

1. Electronics limits PDA output to 0.25 and Technical Gear to 0.75.
2. Polymers then limits Carbon Fiber Mesh and Furniture to 0.375 each.
3. Rough Fibres subsequently limits Furniture to 0.25.
4. Frozen MRE leaves 0.125 Polymers unused despite Carbon Fiber Mesh having spare capacity and Carbon Strands available.

The test asserts that increasing Carbon Fiber Mesh from 0.375 to 0.5 is feasible without decreasing any other output. The existing solver returns 0.5 with the same other terminal outputs.

This tests a particular fixed traversal, not the impossibility of all two-pass algorithms. In particular, its downward sweep has not solved how to finalize shared-resource MREs independently of traversal order. It also does not implement a complete downstream relevance and surplus-priority calculation. It must not replace the working solver or be described as a complete implementation of those semantics. No SBF CU measurement has been made for this incorrect candidate.
