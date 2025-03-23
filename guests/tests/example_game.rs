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

use battleship_core::{
    Direction, GameState, HitType, Position, RoundCommit, RoundInput, Ship, ShipClass,
};
use battleship_guests::{INIT_ELF, ROUND_ELF};
use risc0_zkvm::{default_executor, ExecutorEnv};

// Run the round function once for each round and confirm the state evolves as expected.
fn run_round(state: &mut GameState, shot: Position, hit_expected: HitType) -> anyhow::Result<()> {
    let input = RoundInput {
        state: state.clone(),
        shot,
    };
    let input_state_commit = state.commit();
    let env = ExecutorEnv::builder().write(&input)?.build()?;
    let execution = default_executor().execute(env, ROUND_ELF)?;
    state.apply_shot(shot);
    let commit = RoundCommit {
        shot,
        hit: hit_expected,
        old_state: input_state_commit,
        new_state: state.commit(),
    };
    assert_eq!(commit, execution.journal.decode()?);

    Ok(())
}

#[test]
fn exmaple_game() -> anyhow::Result<()> {
    // Board
    //  | 0 1 2 3 4 5 6 7 8 9 |
    // 0|                     |
    // 1|       B B B B       |
    // 2|                     |
    // 3|     A               |
    // 4|     A               |
    // 5|     A         S S S |
    // 6|     A               |
    // 7|     A   C     D D   |
    // 8|         C           |
    // 9|         C           |
    let mut state = GameState {
        ships: vec![
            Ship::new(ShipClass::Carrier, (2, 3), Direction::Vertical),
            Ship::new(ShipClass::Battleship, (3, 1), Direction::Horizontal),
            Ship::new(ShipClass::Cruiser, (4, 7), Direction::Vertical),
            Ship::new(ShipClass::Submarine, (7, 5), Direction::Horizontal),
            Ship::new(ShipClass::Destroyer, (7, 7), Direction::Horizontal),
        ],
        pepper: rand::random(),
    };

    // Create a RISC Zero executor, which is a RISC-V emulator with support for RISC Zero syscalls.
    // Use it to run the init program to create a committment to a state with verified validity.
    let env = ExecutorEnv::builder().write(&state)?.build()?;
    let execution = default_executor().execute(env, INIT_ELF)?;
    assert_eq!(state.commit(), execution.journal.decode()?);

    // Example player takes their first shot and misses.
    run_round(&mut state, Position { x: 1, y: 1 }, HitType::Miss)?;

    // Example player hits the carrier and then finds the rest of the ship.
    run_round(&mut state, Position { x: 2, y: 5 }, HitType::Hit)?;
    run_round(&mut state, Position { x: 3, y: 5 }, HitType::Miss)?;
    run_round(&mut state, Position { x: 2, y: 6 }, HitType::Hit)?;
    run_round(&mut state, Position { x: 2, y: 7 }, HitType::Hit)?;
    run_round(&mut state, Position { x: 2, y: 8 }, HitType::Miss)?;
    run_round(&mut state, Position { x: 2, y: 4 }, HitType::Hit)?;
    run_round(
        &mut state,
        Position { x: 2, y: 3 },
        HitType::Sunk(ShipClass::Carrier),
    )?;

    // Example player finds and sinks the cruiser.
    run_round(&mut state, Position { x: 4, y: 9 }, HitType::Hit)?;
    run_round(&mut state, Position { x: 4, y: 8 }, HitType::Hit)?;
    run_round(
        &mut state,
        Position { x: 4, y: 7 },
        HitType::Sunk(ShipClass::Cruiser),
    )?;

    // Example player finds and sinks the destroyer.
    run_round(&mut state, Position { x: 7, y: 2 }, HitType::Miss)?;
    run_round(&mut state, Position { x: 7, y: 7 }, HitType::Hit)?;
    run_round(&mut state, Position { x: 6, y: 7 }, HitType::Miss)?;
    run_round(
        &mut state,
        Position { x: 8, y: 7 },
        HitType::Sunk(ShipClass::Destroyer),
    )?;

    // Example player finds and sinks the submarine.
    run_round(&mut state, Position { x: 8, y: 5 }, HitType::Hit)?;
    run_round(&mut state, Position { x: 7, y: 5 }, HitType::Hit)?;
    run_round(
        &mut state,
        Position { x: 9, y: 5 },
        HitType::Sunk(ShipClass::Submarine),
    )?;

    // Example player finds and sinks the battleship.
    run_round(&mut state, Position { x: 3, y: 1 }, HitType::Hit)?;
    run_round(&mut state, Position { x: 4, y: 1 }, HitType::Hit)?;
    run_round(&mut state, Position { x: 5, y: 1 }, HitType::Hit)?;
    run_round(
        &mut state,
        Position { x: 6, y: 1 },
        HitType::Sunk(ShipClass::Battleship),
    )?;

    Ok(())
}
