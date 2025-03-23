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

use battleship_core::GameState;

fn main() {
    // Read in an initial game state supplied by the player.
    let state: GameState = env::read();

    // Check that all ships are placed, all ships and in bounds, and no ships overlap.
    if !state.check() {
        panic!("Invalid GameState");
    }

    // Write a commitment to the game state to the journal for the verifier to read.
    env::commit(&state.commit());
}
