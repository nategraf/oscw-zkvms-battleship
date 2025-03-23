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

use anyhow::ensure;
use battleship_core::{GameState, HitType, Position, RoundCommit, RoundInput, ShipClass};
use battleship_guests::{INIT_ELF, INIT_ID, ROUND_ELF, ROUND_ID};
use inquire::Text;
use regex::Regex;
use risc0_zkvm::{default_prover, sha::Digest, ExecutorEnv, Receipt};

fn main() -> anyhow::Result<()> {
    // Initialize tracing. In order to view logs, run `RUST_LOG=info cargo run`
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let mut opponent = Opponent::random();

    // Require the opponent to prove that their board state is valid. Verify and store the commit.
    println!("Opponent proving initial board state is valid");
    let receipt = opponent.prove_init()?;
    receipt.verify(INIT_ID)?;
    let mut opponent_state_commit: Digest = receipt.journal.decode()?;

    let mut ship_classes = ShipClass::list().to_vec();
    loop {
        let shot = prompt_for_point()?;

        println!("Opponent proving application of shot {}", shot);
        let receipt = opponent.prove_apply_shot(shot)?;

        receipt.verify(ROUND_ID)?;
        let round_commit: RoundCommit = receipt.journal.decode()?;

        // Update our state commitment that we are storing.
        ensure!(
            opponent_state_commit == round_commit.old_state,
            "opponent did not use the correct state"
        );
        ensure!(
            shot == round_commit.shot,
            "opponent did not use the correct shot"
        );
        opponent_state_commit = round_commit.new_state;

        match round_commit.hit {
            HitType::Miss => println!("Shot at {} is a miss", shot),
            HitType::Hit => println!("You scored a hit at {}", shot),
            HitType::Sunk(ship_class) => {
                println!("You sunk a {:?} with your shot at {}", ship_class, shot);
                if let Some(i) = ship_classes.iter().position(|c| ship_class == *c) {
                    ship_classes.swap_remove(i);
                };
            }
        }

        if ship_classes.is_empty() {
            break;
        }
    }

    println!("You won!");
    Ok(())
}

fn prompt_for_point() -> anyhow::Result<Position> {
    // Create regex for validating coordinates in format "x,y" where x and y are 0-9
    let coord_regex = Regex::new(r"^([0-9]),\s*([0-9])$").unwrap();

    loop {
        // Prompt the user for coordinates
        let input = Text::new(
            "Enter coordinates (x,y) for a point on the 10x10 grid (0-9 for each value):",
        )
        .prompt()?;

        // Try to parse and validate the input
        if let Some(captures) = coord_regex.captures(input.trim()) {
            // Extract x and y values
            if let (Some(x_match), Some(y_match)) = (captures.get(1), captures.get(2)) {
                let x: u32 = x_match.as_str().parse().unwrap(); // Safe to unwrap as regex ensures 0-9
                let y: u32 = y_match.as_str().parse().unwrap();

                // Additional validation (although regex already ensures 0-9)
                if x <= 9 && y <= 9 {
                    return Ok(Position { x, y });
                }
            }
        }

        // If we reach here, input was invalid
        println!(
            "Invalid coordinates! Please enter values as 'x,y' where both x and y are between 0-9."
        );
    }
}

// An opponent with their secret Battleship board that the CLI user will play against.
// This opponent is a stand-in for e.g. another human you'd play over the network.
pub struct Opponent {
    state: GameState,
}

impl Opponent {
    pub fn random() -> Self {
        Self {
            state: rand::random(),
        }
    }

    pub fn prove_init(&self) -> anyhow::Result<Receipt> {
        let env = ExecutorEnv::builder().write(&self.state)?.build()?;
        let prove_info = default_prover().prove(env, INIT_ELF).unwrap();

        Ok(prove_info.receipt)
    }

    pub fn prove_apply_shot(&mut self, shot: Position) -> anyhow::Result<Receipt> {
        let input = RoundInput {
            state: self.state.clone(),
            shot,
        };
        let env = ExecutorEnv::builder().write(&input)?.build()?;
        let prove_info = default_prover().prove(env, ROUND_ELF).unwrap();

        // Also update the state. This tracks the chain of states in the guest.
        self.state.apply_shot(shot);

        Ok(prove_info.receipt)
    }
}
