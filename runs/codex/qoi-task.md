# C → Rust Migration Playbook (Agent-Executable)

A generic, repeatable procedure for migrating a C library to a safe Rust crate
**with a differential oracle proving equivalence**. It applies to any C library
with a callable API, turning the migration into a parameterized workflow an
autonomous agent (Claude Code, Codex, etc.) or a human can run end to end.

The spine of the method: **never trust a port you cannot differentially compare
against the original.** Every phase ends in a machine-checkable gate; do not
advance past a failed gate.

---

## 0. How to use this file

1. Fill in the **Config** block below. Everything in `${...}` is a parameter.
2. Execute phases **in order**. Each phase has *Steps*, *Artifacts*, an **Exit
   gate**, and *On failure*.
3. A gate that says `RUN:` is a command whose success (exit 0) is required. A gate
   that says `CHECK:` is a written artifact that must exist and be correct.
4. Honor every **STOP & ASK** condition (§2). When one fires, surface the issue to
   the human and wait — do not guess.
5. If the agent is in an environment **without the Rust toolchain**, follow the
   *No-toolchain validation* fallback (§Appendix A) at each porting gate instead of
   skipping it.

### Config (fill this in)

```yaml
lib_name:        qoi                                   # -> crates qoi-rs / qoi-sys / qoi-cabi
output_dir:      codex                                 # where to create the ${LIB}-rs-migration/ workspace:
                                                       #   "." (default) = same folder as this playbook
                                                       #   or an absolute/relative path the user selects
upstream_url:    https://github.com/phoboslab/qoi      # vendored file: qoi.h
upstream_pin:    master                                # what was fetched; REPLACE with the exact
                                                       # commit SHA recorded at vendor time (repro
                                                       # needs the SHA, not a moving branch)
license:         MIT                                   # confirmed: SPDX-License-Identifier in qoi.h,
                                                       # © 2021 Dominic Szablewski — vendoring allowed
headers:         ["qoi.h"]                             # single-header library
api_functions:   ["qoi_encode", "qoi_decode"]          # QOI_NO_STDIO drops qoi_read/qoi_write
opaque_types:    ["qoi_desc"]                          # only struct crossing the boundary; qoi_rgba_t
                                                       # is internal (not bound)
allocator:       malloc                                # default QOI_MALLOC; freed via libc::free
determinism:     deterministic_bytes                   # output is a pure function of input pixels
oracle_relation: byte_exact                            # validated: 6000 images, 0 mismatches

---

## 1. Operating principles (non-negotiable)

- **Oracle-driven.** Correctness is *defined* as equivalence to the reference C,
  not as "looks right." Choose the equivalence relation explicitly (Phase 1).
- **Unsafe is confined.** The core crate is `#![forbid(unsafe_code)]`. All `unsafe`
  lives only in the two FFI crates (`-sys` inbound, `-cabi` outbound). This is
  compiler-enforced, not a convention.
- **Rewrite, don't transpile.** Port function-by-function against the *spec/
  semantics* using slices and iterators. A mechanical transpile reproduces pointer
  `unsafe` and defeats the purpose.
- **Total on hostile input.** Decoders/parsers must never panic on adversarial
  bytes — return `Result`, bounds-check every read.
- **Allocator symmetry.** A buffer is freed by the same allocator that made it.
  Never adopt a C `malloc` pointer into a Rust `Vec`; never hand C a Rust buffer
  without a matching `*_free`.
- **Reproducible.** Pin the upstream commit, the toolchain, and the C flags. Commit
  `Cargo.lock` and proptest regressions.
- **Validate continuously.** If you cannot run the target toolchain, validate the
  *logic* another way before claiming success (§Appendix A).
- **Leave no trace.** Track every resource you create *outside*
  `${output_dir}/${LIB}-rs-migration/` (scratch files, temp clones, Docker
  images/containers/volumes, installed tools) in a **cleanup ledger** as you go.
  Phase 10 removes them, keeping costly-to-rebuild ones only with the user's consent.

---

## 2. STOP & ASK conditions

Pause and consult the human (do not improvise) when any hold:

1. **License forbids vendoring** or redistribution of the C source.
2. **Output is non-deterministic** (timestamps, hash-map iteration order, threads,
   floating-point reductions) — byte-exact equivalence is impossible; you must
   agree on a weaker relation first.
3. The C API uses **callbacks, global/mutable state, threads, longjmp, or signal
   handlers** — the FFI and oracle design change materially.
4. The API hands back **caller-provided buffers** or has **ownership rules you
   cannot determine** from headers/docs.
5. The reference and your port **disagree and you cannot localize the cause** after
   a bounded effort — surface the minimal reproducer.
6. Porting would require **reproducing undefined behavior** the C relies on.

---

## 3. Generic workspace skeleton

```
${output_dir}/${LIB}-rs-migration/   # output_dir defaults to the playbook's folder (Phase 0)
├── Cargo.toml                    # workspace: members = core/-sys/-cabi/-diff; exclude = ["fuzz"]
├── README.md                     # report + deliverable mapping + provenance
├── REPRODUCIBILITY.md            # pins: upstream commit, toolchain, C flags
├── LICENSE                       # this project's license
├── rust-toolchain.toml           # stable (note: fuzzing needs nightly)
├── .github/workflows/ci.yml      # gates: build/test, unsafe, fuzz-smoke, bench
├── vendor/${LIB}/                # upstream source @ ${COMMIT}, verbatim, + LICENSE
├── crates/
│   ├── ${LIB}-rs/                # SAFE core  (#![forbid(unsafe_code)], zero deps)
│   │   ├── src/lib.rs
│   │   ├── tests/golden.rs + golden_data.rs   # known-answer, no C needed
│   │   ├── benches/bench.rs                    # criterion
│   │   └── examples/bench_bin.rs               # hyperfine subject (Rust side)
│   ├── ${LIB}-sys/               # FFI IN: cc + bindgen over vendored C
│   │   ├── build.rs  wrapper.h  ${LIB}_impl.c
│   │   └── src/lib.rs                           # owned-Vec wrappers; allocator-symmetric free
│   ├── ${LIB}-cabi/              # FFI OUT: extern "C" + cbindgen + *_free + RAII
│   │   ├── build.rs  cbindgen.toml
│   │   └── src/lib.rs
│   └── ${LIB}-diff/              # differential oracle (proptest) + shared Input model
│       ├── src/lib.rs
│       └── tests/differential.rs
├── fuzz/                         # cargo-fuzz (own workspace; nightly)
│   └── fuzz_targets/{differential.rs, no_panic.rs}
├── bench/${LIB}_cbench.c         # hyperfine subject (C side, -O3)
└── scripts/{bench.sh, check_unsafe.sh}
```

Naming: `${LIB}-rs` (core), `${LIB}-sys` (inbound FFI / ground truth), `${LIB}-cabi`
(outbound FFI), `${LIB}-diff` (oracle).

---

## Phase 0 — Intake & environment

**Goal:** lock parameters; know what you can execute.

**Steps**
- Complete the Config block.
- **Resolve `output_dir`:** if unset or `"."`/`"alongside"`, create the workspace in
  the same folder as this playbook (default); otherwise use the absolute/relative
  path the user gave. If the user has not indicated a preference, ask which of the
  two they want before scaffolding. Expand `~`, create the directory if missing, and
  confirm it is writable. The workspace root is `${output_dir}/${LIB}-rs-migration/`.
- Probe the environment and record results:
  `rustc --version; cargo --version; cc --version; (clang/libclang present?); cargo fuzz --version; hyperfine --version`.
- Set **validation mode**: `full` (cargo available) or `no_toolchain` (use §Appendix A).

**Exit gate**
- CHECK: Config block fully filled.
- CHECK: `output_dir` resolved to a concrete, writable absolute path and recorded;
  `${output_dir}/${LIB}-rs-migration/` is where all later phases write.
- CHECK: environment + validation mode recorded in `REPRODUCIBILITY.md`.

**On failure:** if a parameter is unknown, gather it from the upstream repo/docs
before proceeding; do not leave `${...}` placeholders downstream.

---

## Phase 1 — Characterize the C library & choose the oracle  *(most important phase)*

**Goal:** understand semantics well enough to define *equivalence*.

**Steps**
1. **Vendor** the source at `${COMMIT}` into `vendor/${LIB}/`, verbatim, with its
   `LICENSE`. Confirm the license permits this (§2.1).
2. **API inventory:** for each `${FUNCS}` record signature, parameter directions
   (in/out/inout), and preconditions.
3. **Ownership table:** for every pointer crossing the boundary, record who
   allocates and who frees, and with which allocator (`${ALLOC}`).
4. **Error model:** return codes vs `errno` vs out-params vs sentinel/NULL.
5. **Determinism:** is output a pure function of input bytes? Note any nondeterminism
   (→ §2.2).
6. **Edge cases:** enumerate boundary conditions (empty input, max sizes, integer
   overflow guards, wrapping arithmetic, alignment). Read the reference
   implementation, not just the header.
7. **Choose `oracle_relation`:**
   - `byte_exact` — pure deterministic transforms (codecs, hashers, serializers).
     Strongest; assert output buffers are identical.
   - `behavioral` — equivalence = (same return code) ∧ (same out-params/buffer) ∧
     (same error signal). Use when output isn't uniquely defined but observably
     equal.
   - `model_based` — for stateful APIs: drive both with the same operation sequence
     and compare observable state after each step.

**Artifacts:** `vendor/${LIB}/`, an API + ownership + edge-case note (put it in the
README or a `NOTES.md`), the chosen relation in Config.

**Exit gate**
- CHECK: ownership table complete (no "unknown" rows) — else STOP (§2.4).
- CHECK: `oracle_relation` chosen and justified.
- CHECK: edge-case list exists and informs the generators in Phase 5.

---

## Phase 2 — Scaffold workspace

**Goal:** compiling skeleton with the crate boundary in place.

**Steps:** create the §3 tree and the manifests. Core crate: `#![forbid(unsafe_code)]`
and **no dependencies**. Workspace `exclude = ["fuzz"]` (it needs nightly).

**Exit gate**
- RUN (full): `cargo metadata --no-deps >/dev/null`
- CHECK (no_toolchain): tree + manifests present and internally consistent.

---

## Phase 3 — FFI-in (`-sys`) + oracle ground truth

**Goal:** call the *real* C from Rust; this is what the port is compared against.

**Steps**
1. `build.rs`: compile the vendored C with `cc` at **`-O3`**; generate bindings with
   `bindgen` (allowlist only `${FUNCS}` + `${TYPES}`). (Template: Appendix B.)
2. `wrapper.h` for bindgen; `${LIB}_impl.c` as the single C translation unit.
3. Safe wrappers that **copy** the C result into an owned `Vec`/struct and free the C
   buffer with the **matching** allocator (`libc::free` for `malloc`). Never
   `Vec::from_raw_parts` a C pointer. (Template: Appendix C.)

**Exit gate**
- RUN (full): `cargo test -p ${LIB}-sys` — a reference round-trip passes.
- CHECK (no_toolchain): §Appendix A — C harness exercising the reference passes.

**On failure:** bindgen needs `libclang` (`apt-get install libclang-dev`). If the C
won't compile, check include paths and feature `#define`s.

---

## Phase 4 — Safe Rust port (`-rs` core)

**Goal:** idiomatic, panic-free, dependency-free reimplementation.

**Steps**
- Port each function against the spec using slices/iterators. Mirror integer
  semantics exactly: C `unsigned char` subtraction → `u8::wrapping_sub(..) as i8`;
  truncating assignment → `as u8`. Compute hashes/indices in a wide type then reduce.
- Validate inputs in the public API (lengths, dimensions, overflow guards mirroring
  the reference's caps).
- Make all reads bounds-checked (a cursor returning a default past end keeps the
  decoder total while staying bit-identical on valid input).
- Add a typed `Error` enum (`Display` + `std::error::Error`), no `panic!`/`unwrap`
  on the hot path.
- Generate **golden vectors** from the reference (Phase 3) and check them in as
  `tests/golden_data.rs` so the core is testable with **no C present**.

**Exit gate**
- RUN (full): `cargo test -p ${LIB}-rs` (unit + golden) passes.
- CHECK (no_toolchain): §Appendix A confirms the mirror matches the reference, and
  golden vectors were produced from the reference encoder.

---

## Phase 5 — Differential oracle (proptest)

**Goal:** prove equivalence across a large, path-covering input space.

**Steps**
- Put a shared `Input` model + `check_against_reference(input)` in
  `${LIB}-diff/src/lib.rs` (reused by fuzz). It asserts the chosen relation:
  - round-trip on the Rust side,
  - relation(Rust output, C output) holds (byte-exact ⇒ identical),
  - cross-consume: each side consumes the other's output to the original,
  - structural invariants checkable **without** either impl (magic/header/sizes).
- In `tests/differential.rs`, write generators **biased to hit every code path**
  (one strategy per opcode/branch identified in Phase 1) plus an arbitrary-bytes
  strategy and explicit boundary cases. (Template: Appendix D.)

**Exit gate**
- RUN (full): `cargo test -p ${LIB}-diff` with `ProptestConfig::cases(>=1024)` passes.
- On counterexample: proptest shrinks it — fix the port, commit the regression seed.

---

## Phase 6 — Fuzzing

**Goal:** find divergences and panics the structured tests miss.

**Steps**
- `fuzz/fuzz_targets/differential.rs`: coerce arbitrary bytes into a small valid
  input (mask sizes to bound memory) → `check_against_reference`.
- `fuzz/fuzz_targets/no_panic.rs`: feed adversarial bytes straight to the
  decoder/parser (mask header sizes to avoid OOM) — property is *no panic*; if it
  succeeds, a re-encode round-trip must reproduce the output. (Template: Appendix E.)

**Exit gate**
- RUN: `cargo +nightly fuzz run differential -- -max_total_time=${T:-60}` — no crash.
- RUN: `cargo +nightly fuzz run no_panic -- -max_total_time=${T:-60}` — no crash.

---

## Phase 7 — FFI-out (`-cabi`)

**Goal:** expose the safe Rust port to C with a stable, safe-to-call ABI.

**Steps**
- `#[no_mangle] extern "C"` entry points; `#[repr(C)]` structs; fixed-width ints
  (`u32`/`u8`/`c_int`) for ABI stability across 32/64-bit.
- A `${LIB}_rs_free(ptr, len)` that reconstructs the boxed slice with the **same
  length** so Rust's allocator frees exactly what it allocated. Document that buffers
  from `*_encode`/`*_decode` must be released via this function. (Template: Appendix F.)
- `cbindgen` generates `include/${LIB}_rs.h` in `build.rs` (non-fatal on failure).
- Provide a Rust-side test that drives the C ABI end to end (`encode→decode→free`)
  and checks bytes equal the safe API — proves the boundary without dlopen.

**Exit gate**
- RUN (full): `cargo test -p ${LIB}-cabi` passes; `include/${LIB}_rs.h` generated.

---

## Phase 8 — Benchmarks, reproducibility, CI

**Goal:** quantify, pin, and automate.

**Steps**
- **criterion** microbenchmarks (throughput in ops/s or bytes/s) across the Phase-1
  patterns.
- **hyperfine** whole-process comparison: `bench/${LIB}_cbench.c` (built `-O3`) vs
  `examples/bench_bin` (built `--release`), identical CLI + work + a printed checksum
  to defeat dead-code elimination. Driver in `scripts/bench.sh`.
- **REPRODUCIBILITY.md:** upstream `${COMMIT}`, Rust channel pin, C flags (`-O3`),
  AI model snapshots if agent-driven, commit `Cargo.lock`.
- **CI** (`.github/workflows/ci.yml`) jobs:
  - `build-test` (stable; `apt install libclang-dev`; `cargo build/test --workspace`),
  - `unsafe-gate` (`scripts/check_unsafe.sh`; the `forbid` makes it compile-enforced),
  - `fuzz-smoke` (nightly; `cargo fuzz run … -max_total_time=20` per target),
  - `bench` (criterion short sample + `scripts/bench.sh` small).
  Keep `fmt`/`clippy` **informational** until the tree is verified, then tighten to
  `-D warnings`.

**Exit gate**
- RUN: full CI pipeline green (or local equivalents of every job).

---

## Phase 9 — Human-readable handoff summary

**Goal:** produce a plain-language summary and a copy-paste manual run/test guide.
Emit it **twice**: (a) write `SUMMARY.md` at the repo root (committed, for later
readers), and (b) reproduce the same content as the agent's final message to the
user.

**Steps**
1. Fill the template below with the real Config values and the **actual** results
   from Phases 4–8 (test counts, fuzz duration, benchmark numbers, `unsafe` block
   count + locations). Do not invent numbers — only report what was run.
2. Write it to `SUMMARY.md`.
3. Reproduce it verbatim in the agent's closing response.

**Template** (`SUMMARY.md` — replace every `${...}` and `<fill>`; nothing may
survive unfilled):

````markdown
# ${LIB}: C → Rust migration — summary

## What this is
A safe Rust reimplementation of the C library **${LIB}**
(`${UPSTREAM_URL}` @ `${upstream_pin}`, ${license}), verified to match the original
by a differential oracle. The algorithm is `#![forbid(unsafe_code)]`; all FFI
`unsafe` is isolated in two boundary crates.

## Crates
| Crate | What it is | unsafe |
|---|---|---|
| `${LIB}-rs`   | Safe core port — **use this** | no |
| `${LIB}-sys`  | Bindings to the original C (test ground truth) | yes (FFI) |
| `${LIB}-cabi` | C ABI exposing the Rust port | yes (FFI) |
| `${LIB}-diff` | Differential tests (Rust vs C) | no |

## Equivalence guarantee
- Relation checked: **${oracle_relation}** over functions ${FUNCS}.
- Verified by: <fill: e.g. "2048 proptest cases + N golden vectors + T s fuzzing">.
- Result: **<fill: all pass / mismatches found and fixed>**.

## Safety
- Core (`${LIB}-rs`): zero `unsafe`, enforced at compile time by
  `#![forbid(unsafe_code)]`.
- `unsafe` appears only in `${LIB}-sys` and `${LIB}-cabi` (FFI): <fill: N> blocks,
  each with a documented safety contract.

## Prerequisites
- Rust (stable) + Cargo; a C compiler (`cc`); **libclang** for bindgen
  (`sudo apt-get install -y libclang-dev`).
- Optional: nightly + `cargo install cargo-fuzz` (fuzzing); `hyperfine` (benchmarks).

## Build, test, run — manually
```bash
# build everything
cargo build --workspace

# full test suite: unit + golden vectors + differential (Rust vs C)
cargo test --workspace

# core only — needs NO C toolchain / libclang
cargo test -p ${LIB}-rs

# confirm the core has no unsafe
bash scripts/check_unsafe.sh

# fuzz (nightly): runs until Ctrl-C, or cap with -max_total_time
cargo +nightly fuzz run differential -- -max_total_time=60
cargo +nightly fuzz run no_panic     -- -max_total_time=60

# benchmarks: in-process throughput, then C(-O3) vs Rust(--release) head-to-head
cargo bench -p ${LIB}-rs
bash scripts/bench.sh
```
What each proves: `cargo test` = correctness vs the reference; `check_unsafe.sh` =
safety boundary intact; `fuzz` = no divergence/panic on hostile input; `bench` =
performance parity.

## Results snapshot
- Tests: <fill: X passed, 0 failed>.
- Fuzzing: <fill: which targets, duration, crashes found>.
- Benchmark: <fill: e.g. "process 0.9–1.1× the C reference">.

## Known limitations / TODO
- <fill: e.g. streaming API not ported; SIMD path TODO; record exact commit SHA in REPRODUCIBILITY.md>.

## Provenance
Upstream `${UPSTREAM_URL}` @ `${upstream_pin}` (${license}), vendored unmodified in
`vendor/${LIB}/`. Migration performed per the C → Rust migration playbook.
````

**Exit gate**
- CHECK: `SUMMARY.md` exists at the repo root and contains **no** surviving
  `${...}` or `<fill>` tokens.
- CHECK: every command listed was actually executed, or is annotated as optional /
  not run.
- CHECK: the agent's final response reproduces the summary.

---

## Phase 10 — Clean up after the run

**Goal:** leave the machine as it was found, keeping only the deliverables in
`${output_dir}/${LIB}-rs-migration/`. Run this **last**, after `SUMMARY.md` is
written and the deliverables are in place.

**Steps**
1. Walk the **cleanup ledger** (the running list from §1 of everything created
   outside the workspace) and classify each item:
   - **Deliverable** (inside `${output_dir}/${LIB}-rs-migration/`) → keep, always.
   - **Transient** → remove now, no prompt. E.g. scratch/temp dirs, `/tmp` files, the
     throwaway C-mirror harness (§Appendix A), shallow clones used only to read
     upstream, stray object files outside the workspace.
   - **Keep-with-consent** (costly to recreate) → list each with its size /
     recreate-cost and **ask the user**; remove only those they decline. E.g. Docker
     images/containers/volumes/networks built for the run, downloaded source
     tarballs, fuzz corpora, the `target/` build cache, toolchains/components
     installed solely for this task.
2. Delete the transient items. Then present the keep-with-consent list and **wait**
   for the user's choice; delete the declined ones, retain the approved ones.
3. Report the outcome: append a `## Cleanup` section to `SUMMARY.md` (what was
   removed, what was kept and why) and state it in the closing message.

**Scope guard (safety)**
- Only ever remove resources **this run created**, identified by the exact
  names/paths/labels you recorded — never a broad sweep.
- Do **not** run global purges (`docker system prune -a`, cleaning shared caches,
  `rm -rf` on shared dirs). Make cleanup exact by tagging/naming what you create up
  front, e.g. Docker images `${LIB}-migration:*` and containers `${LIB}-mig-*`.
- If you cannot prove you created something, treat it as **keep-with-consent** (ask),
  never as transient.

**Exit gate**
- CHECK: every transient ledger item is gone; `${output_dir}/${LIB}-rs-migration/`
  is intact.
- CHECK: each keep-with-consent item was either user-approved to keep or removed —
  **nothing deleted without consent**, nothing pre-existing touched.
- CHECK: cleanup outcome recorded in `SUMMARY.md` and the closing message.

---

## Final deliverable checklist

- [ ] Vendored C @ pinned commit + license; ownership/edge-case notes.
- [ ] `${LIB}-rs` core: `#![forbid(unsafe_code)]`, zero deps, typed errors, total on
      bad input.
- [ ] Golden vectors generated from the reference, checked in, pass with no C.
- [ ] `${LIB}-sys` FFI-in via cc+bindgen; allocator-symmetric frees.
- [ ] `${LIB}-cabi` FFI-out: `extern "C"`, `repr(C)`, `*_free`, cbindgen header.
- [ ] `${LIB}-diff` proptest (≥1024 cases) on the chosen relation — green.
- [ ] Two fuzz targets run clean for ≥ T seconds.
- [ ] criterion + hyperfine (C `-O3` vs Rust `--release`).
- [ ] Reproducibility manifest + `Cargo.lock` + CI green.
- [ ] README maps each deliverable to its location and records provenance.
- [ ] `SUMMARY.md` written (human summary + manual run/test guide), placeholder-free, and echoed to the user.
- [ ] Cleanup done (Phase 10): transient artifacts removed; Docker/caches/other costly resources kept only with user consent; outcome noted in `SUMMARY.md`.

---

## Adaptation guide (when the library isn't a clean pure-function API)

| Situation | Adjustment |
|---|---|
| **Nondeterministic output** | Use `behavioral`/`model_based`; assert decode(Rust)≡decode(C) or invariants, not byte equality. (§2.2 — agree first.) |
| **Opaque handles / stateful** | `model_based`: generate operation sequences with `proptest`; step both impls; compare observable state each step. RAII wrapper owns the handle, `Drop` calls the C destructor. |
| **Caller-provided buffers** | No allocation crosses the boundary; assert the bytes written + the returned length match. |
| **Error codes / errno** | Equivalence includes the error signal; map C codes to a Rust `Error` enum and assert both the value and the error. |
| **Callbacks / global state / threads** | STOP & ASK (§2.3); design the FFI and oracle with the human before porting. |
| **Floating point** | Exact bit-equality may be unattainable across compilers; agree on ULP tolerance or compare against a reference oracle, not C-vs-Rust directly. |
| **Huge/streaming inputs** | Bound generator/fuzz sizes; add `try_reserve` and size caps mirroring the reference's guards. |

---

## Appendix A — Validation without the target toolchain

If `cargo`/`rustc` is unavailable but a C compiler is, **de-risk the port logic
before a compiler exists**:

1. Write a C transliteration that is *structurally identical* to the intended Rust
   (same control flow, same variable names, same integer arithmetic — e.g. model
   `u8::wrapping_sub(..) as i8` with `(int8_t)(uint8_t)(a-b)`).
2. Compile it together with the **real** vendored C reference.
3. Fuzz both over thousands of inputs covering every branch identified in Phase 1;
   assert the relation (byte-exact / behavioral). Zero divergences ⇒ the *algorithm*
   is correct; the only residual risk is Rust compile/syntax.
4. Generate golden vectors from the reference in the same harness so the eventual
   Rust tests assert against authentic bytes.

This mirror-and-diff approach front-loads confidence before any Rust runs. It does
not replace `cargo test` — it is the required substitute at the Phase 3/4 gates in
`no_toolchain` mode.

## Appendix B — `-sys` build.rs (cc + bindgen)

```rust
let vendor = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/${LIB}");
println!("cargo:rerun-if-changed=${LIB}_impl.c");
cc::Build::new().file("${LIB}_impl.c").include(&vendor).opt_level(3).warnings(false)
    .compile("${LIB}_reference");
let bindings = bindgen::Builder::default()
    .header("wrapper.h").clang_arg(format!("-I{}", vendor.display()))
    .allowlist_function("${FUNC}").allowlist_type("${TYPE}")
    .layout_tests(false).generate().unwrap();
bindings.write_to_file(PathBuf::from(env!("OUT_DIR")).join("bindings.rs")).unwrap();
// Cargo.toml: links = "${LIB}_reference"; [build-dependencies] cc, bindgen; [dependencies] libc
```

## Appendix C — Allocator-symmetric inbound free

```rust
unsafe {
    let ptr = ffi_encode(input.as_ptr().cast(), &desc, &mut out_len); // C malloc'd
    if ptr.is_null() { return None; }
    let v = std::slice::from_raw_parts(ptr as *const u8, out_len as usize).to_vec(); // copy out
    libc::free(ptr);                                                                 // C free
    Some(v)
}
```

## Appendix D — proptest differential skeleton

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]
    #[test]
    fn differential(input in path_covering_strategy()) {
        prop_assert!(check_against_reference(&input).is_ok(),
                     "{}", check_against_reference(&input).unwrap_err());
    }
}
// path_covering_strategy(): prop_oneof! one generator per branch found in Phase 1,
// plus prop::collection::vec(any::<u8>(), ..) for arbitrary inputs.
```

## Appendix E — fuzz targets

```rust
// differential.rs
fuzz_target!(|data: &[u8]| {
    if let Some(inp) = Input::coerce(/* mask sizes from data */) {
        if let Err(e) = check_against_reference(&inp) { panic!("mismatch: {e}"); }
    }
});
// no_panic.rs : mask header size fields, then `let _ = decode(&buf);` must not panic;
// if Ok, a re-encode round-trip must reproduce the output.
```

## Appendix F — Outbound `extern "C"` + symmetric free

```rust
#[no_mangle] pub unsafe extern "C" fn ${LIB}_rs_encode(
    ptr: *const u8, desc: *const ${LIB}RsDesc, out_len: *mut c_int) -> *mut u8 {
    /* null-check; validate desc; build slice; call safe encode() */
    let bytes = match encode(input, &rdesc) { Ok(v) => v, Err(_) => return null_mut() };
    let len = bytes.len(); *out_len = len as c_int;
    Box::into_raw(bytes.into_boxed_slice()).cast()        // Rust-allocated
}
#[no_mangle] pub unsafe extern "C" fn ${LIB}_rs_free(ptr: *mut u8, len: c_int) {
    if ptr.is_null() || len <= 0 { return; }
    drop(Box::from_raw(slice_from_raw_parts_mut(ptr, len as usize))); // same len ⇒ same allocator
}
// [lib] crate-type = ["cdylib","staticlib","rlib"]; build.rs runs cbindgen.
```

---

*Fill the Config block, then execute the phases in order. Every `${...}` token is a
parameter; nothing in this playbook is specific to a particular library.*
