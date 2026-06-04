# Formal verification

Formal verification is enabled with Kani for a first pass over pure core helper
logic. It does not verify RE2's C++ implementation, the FFI boundary, bindgen
output, system `libre2`, or destructor execution in C++.

## Harnesses

| Harness | Scope | Bounds | Invariants |
| --- | --- | --- | --- |
| `regex_handle_preserves_lifetime_invariants` | Pure ownership-state model used to document the safe wrapper lifecycle | unwind 8 | successful construction has a live handle; invalid construction has no live handle; free is idempotent in the model |
| `partial_match_no_panic` | Pure input-bound helper | max pattern len 32, max text len 128, unwind 8 | accepted lengths stay within configured bounds |

## Command

```sh
bash scripts/verify_kani.sh
```

The script runs:

```sh
cargo +stable kani -p re2-cpp-rs --no-default-features
```

## Result

Docker validation on 2026-06-04 installed `kani-verifier v0.67.0` with the
stable Rust toolchain and verified both configured harnesses successfully:

- `verification::regex_handle_preserves_lifetime_invariants`
- `verification::partial_match_no_panic`

## Limitations

These are bounded proof harnesses. The behavioral oracle and fuzz targets remain
responsible for exercising the real C++ RE2 facade.
