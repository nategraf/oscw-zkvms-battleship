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
