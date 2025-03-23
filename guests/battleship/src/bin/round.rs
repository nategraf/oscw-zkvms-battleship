use risc0_zkvm::guest::env;

use battleship_core::{RoundCommit, RoundInput};

fn main() {
    // Read in the current same state and the shot to apply.
    let RoundInput { mut state, shot } = env::read();

    // Commit to the state before applying the shot, apply the shot and then commit to the state
    // after applying the shot.
    let old_state_commit = state.commit();
    let hit = state.apply_shot(shot);
    let new_state_commit = state.commit();

    // Commit the results to be read by the verifier.
    env::commit(&RoundCommit {
        old_state: old_state_commit,
        new_state: new_state_commit,
        shot,
        hit,
    });
}
