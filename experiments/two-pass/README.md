# Batched MRE experiment

Run `node run.mjs --cu` from this directory. The runner copies the shared Rust engine into a temporary directory, replaces only its allocation routine, and routes full calculations through that routine. The working engine and browser binaries are unchanged. Temporary build files and unfunded program keypairs are removed in `finally`. SBF measurements use the existing local Mollusk runner with its unchanged 200,000 CU limit.

For each existing priority batch:

1. Establish each output's individual maximum from all expanded supplier constraints. This accounts for known bottlenecks before sharing inputs.
2. Collect limits in an MRE array, without updating published allocation values or constraint residuals.
3. For each shared-input constraint, derive its capacity-weighted allocation threshold using the current MRE caps. Sort capped-efficiency breakpoints and evaluate them directly; no numerical convergence.
4. Choose the lowest threshold, lower the affected MREs, and retire that constraint. Refresh cached thresholds only for remaining constraints intersecting changed MREs. Each constraint is retired at most once.
5. Apply the resulting MREs together, updating the allocation values and residuals once for that batch.

Stock handling, alternative-producer recovery and recipe expressions come from the existing engine. Priorities remain the existing final-output-first and descending-depth batches. This tests batching application of limits; it does not establish that two fixed-order traversals alone are sufficient to determine those limits. The initially tested fixed-order collection left usable inputs unconsumed.

The correctness harness includes the known local bottleneck control, 205 fixtures and 1,000 generated mixed capacity/stock configurations. It compares all recipe rates with the baseline at a tolerance of 128 Q24 ticks and checks for factories with available inputs that could increase production without reducing another factory. It is not a general proof of optimality. `results.txt` records the observed maximum difference.

`results-cu.json` contains actual normal and experimental SBF instruction costs for five representative cases. A failed 200,000-CU execution means the cost exceeded the budget; it is not an exact measurement of how much work remained. The prototype improves some cases and exceeds the limit in others, so it is not enabled in the working engine.
