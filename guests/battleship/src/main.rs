use risc0_zkvm::guest::env;

use battleship_core::GameState;

fn main() {
    let state: GameState = env::read();
    if !state.check() {
        panic!("Invalid GameState");
    }
    env::commit(&state);
}
