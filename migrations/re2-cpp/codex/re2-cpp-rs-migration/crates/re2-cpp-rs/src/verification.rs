use super::{HandleLifecycle, InputBounds};

#[kani::proof]
#[kani::unwind(8)]
fn regex_handle_preserves_lifetime_invariants() {
    let success: bool = kani::any();
    let mut state = HandleLifecycle::construct(success);
    assert!(state.invariant_holds());

    if success {
        assert!(state.live());
    } else {
        assert!(!state.live());
    }

    state.free();
    assert!(!state.live());
    assert!(state.invariant_holds());
}

#[kani::proof]
#[kani::unwind(8)]
fn partial_match_no_panic() {
    let pattern_len: usize = kani::any();
    let text_len: usize = kani::any();
    kani::assume(pattern_len <= 32);
    kani::assume(text_len <= 128);

    let bounds = InputBounds::verified();
    assert!(bounds.accepts(pattern_len, text_len));
}
