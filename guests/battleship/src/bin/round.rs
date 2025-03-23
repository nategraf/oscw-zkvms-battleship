// Copyright 2025 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
