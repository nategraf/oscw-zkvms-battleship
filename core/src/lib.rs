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

use std::fmt::Display;

#[cfg(feature = "rand")]
use rand::{
    distr::{Distribution, StandardUniform},
    seq::SliceRandom,
    Rng,
};
use serde::{Deserialize, Serialize};

use risc0_zkvm::sha::Digest;

pub const NUM_SHIPS: usize = 5;
pub const BOARD_SIZE: usize = 10;

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Hash)]
pub enum ShipClass {
    Carrier,
    Battleship,
    Cruiser,
    Submarine,
    Destroyer,
}

impl ShipClass {
    pub fn span(&self) -> u32 {
        match self {
            ShipClass::Carrier => 5,
            ShipClass::Battleship => 4,
            ShipClass::Cruiser => 3,
            ShipClass::Submarine => 3,
            ShipClass::Destroyer => 2,
        }
    }

    pub fn sunk_mask(&self) -> u8 {
        (1u8 << self.span()) - 1
    }

    pub const fn list() -> &'static [ShipClass] {
        &[
            Self::Carrier,
            Self::Battleship,
            Self::Cruiser,
            Self::Submarine,
            Self::Destroyer,
        ]
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Hash)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Ship {
    pub class: ShipClass,
    pub pos: Position,
    pub dir: Direction,
    pub hit_mask: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct GameState {
    pub ships: Vec<Ship>,
    /// Entropy added to the game state such that the commitment is hiding.
    pub pepper: [u8; 16],
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct RoundInput {
    pub state: GameState,
    pub shot: Position,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub enum HitType {
    Miss,
    Hit,
    Sunk(ShipClass),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct RoundOutput {
    pub state: GameState,
    pub hit: HitType,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RoundCommit {
    pub old_state: Digest,
    pub new_state: Digest,
    pub shot: Position,
    pub hit: HitType,
}

impl Ship {
    pub fn points(&self) -> impl Iterator<Item = Position> + use<'_> {
        (0..self.class.span()).map(|offset| self.pos.step(self.dir, offset))
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.points().any(|p| other.points().any(|q| p == q))
    }

    pub fn in_bounds(&self) -> bool {
        self.pos.in_bounds() && self.pos.step(self.dir, self.class.span()).in_bounds()
    }
}

impl GameState {
    pub fn new(pepper: [u8; 16]) -> Self {
        Self {
            ships: Vec::new(),
            pepper,
        }
    }

    /// Checks whether the game state contains a valid configuration of ships.
    #[must_use]
    pub fn check(&self) -> bool {
        // Ensure every ship is in bounds.
        for ship in self.ships.iter() {
            if !ship.in_bounds() {
                return false;
            }
        }

        // Ensure every ship class appears exactly once.
        let mut classes = ShipClass::list().to_vec();
        for ship in self.ships.iter() {
            let Some(class_index) = classes.iter().position(|class| ship.class == *class) else {
                return false;
            };
            classes.swap_remove(class_index);
        }
        if !classes.is_empty() {
            return false;
        }

        // Ensure no two ships are intersecting.
        for (i, ship_i) in self.ships.iter().enumerate() {
            for ship_j in self.ships.iter().skip(i + 1) {
                if ship_i.intersects(ship_j) {
                    return false;
                }
            }
        }

        true
    }

    #[must_use]
    pub fn add(&mut self, new_ship: Ship) -> bool {
        if !new_ship.in_bounds() {
            return false;
        }

        // Ensure that there is not already a ship with that class in the state.
        for ship in self.ships.iter() {
            if ship.class == new_ship.class {
                return false;
            }
            if ship.intersects(&new_ship) {
                return false;
            }
        }

        true
    }

    pub fn apply_shot(&mut self, shot: Position) -> HitType {
        for ship in self.ships.iter_mut() {
            let hit = ship.apply_shot(shot);
            match hit {
                HitType::Hit | HitType::Sunk(_) => return hit,
                HitType::Miss => continue,
            }
        }
        HitType::Miss
    }
}

impl Distribution<GameState> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GameState {
        // Create a shuffled list of all positions on the board.
        let mut positions: Vec<Position> = (0..BOARD_SIZE)
            .zip(0..BOARD_SIZE)
            .map(|(x, y)| Position {
                x: x as u32,
                y: y as u32,
            })
            .collect();
        positions.shuffle(rng);

        // Place the ships from largest to smallest, and using the shuffled positions.
        let mut state = GameState::new(rng.random());
        for ship_class in ShipClass::list() {
            for pos in positions.iter() {
                let dir = rng.random();
                if state.add(Ship::new(*ship_class, *pos, dir)) {
                    break;
                }
                if state.add(Ship::new(*ship_class, *pos, dir.flip())) {
                    break;
                }
            }
        }

        // The resulting state should always be valid.
        assert!(state.check());
        state
    }
}

impl Position {
    pub fn step(self, dir: Direction, dist: u32) -> Self {
        match dir {
            Direction::Vertical => Self {
                x: self.x,
                y: self.y + dist,
            },
            Direction::Horizontal => Self {
                x: self.x + dist,
                y: self.y,
            },
        }
    }

    /// Check that the [Position] is within the bounds of the board.
    #[must_use]
    pub fn in_bounds(&self) -> bool {
        self.x < BOARD_SIZE as u32 && self.y < BOARD_SIZE as u32
    }
}

impl From<(u32, u32)> for Position {
    fn from(value: (u32, u32)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Direction {
    pub fn flip(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

impl Distribution<Direction> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        match rng.random::<bool>() {
            true => Direction::Horizontal,
            false => Direction::Vertical,
        }
    }
}

impl Ship {
    pub fn new(class: ShipClass, pos: impl Into<Position>, dir: Direction) -> Self {
        Ship {
            class,
            pos: pos.into(),
            dir,
            hit_mask: 0,
        }
    }

    pub fn with_hit_mask(self, hit_mask: u8) -> Self {
        Self { hit_mask, ..self }
    }

    pub fn apply_shot(&mut self, shot: Position) -> HitType {
        let hit_index = self.points().position(|pos| pos == shot);
        match hit_index {
            Some(hit_index) => {
                self.hit_mask |= 1 << hit_index;
                match self.hit_mask == self.class.sunk_mask() {
                    true => HitType::Sunk(self.class),
                    false => HitType::Hit,
                }
            }
            None => HitType::Miss,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let state = GameState {
            ships: vec![
                Ship::new(ShipClass::Carrier, (2, 3), Direction::Vertical),
                Ship::new(ShipClass::Battleship, (3, 1), Direction::Horizontal),
                Ship::new(ShipClass::Cruiser, (4, 7), Direction::Vertical),
                Ship::new(ShipClass::Submarine, (7, 5), Direction::Horizontal),
                Ship::new(ShipClass::Destroyer, (7, 7), Direction::Horizontal),
            ],
            pepper: rand::random(),
        };

        assert!(state.check());
    }

    #[test]
    fn overlap() {
        let state = GameState {
            ships: vec![
                Ship::new(ShipClass::Carrier, (2, 3), Direction::Vertical),
                Ship::new(ShipClass::Battleship, (3, 1), Direction::Horizontal),
                Ship::new(ShipClass::Cruiser, (2, 3), Direction::Vertical),
                Ship::new(ShipClass::Submarine, (7, 5), Direction::Horizontal),
                Ship::new(ShipClass::Destroyer, (7, 7), Direction::Horizontal),
            ],
            pepper: rand::random(),
        };

        assert!(!state.check());
    }

    #[test]
    fn rounds() {
        // Board
        //  | 0 1 2 3 4 5 6 7 8 9 |
        // 0|                     |
        // 1|       B B B B       |
        // 2|                     |
        // 3|     A               |
        // 4|     A               |
        // 5|     A         D D D |
        // 6|     A               |
        // 7|     A   C     E E   |
        // 8|         C           |
        // 9|         C           |

        let pepper = rand::random();
        let mut state = GameState {
            ships: vec![
                Ship::new(ShipClass::Carrier, (2, 3), Direction::Vertical),
                Ship::new(ShipClass::Battleship, (3, 1), Direction::Horizontal),
                Ship::new(ShipClass::Cruiser, (4, 7), Direction::Vertical),
                Ship::new(ShipClass::Submarine, (7, 5), Direction::Horizontal),
                Ship::new(ShipClass::Destroyer, (7, 7), Direction::Horizontal),
            ],
            pepper,
        };

        // Round 1
        let state_expected = state.clone();
        assert_eq!(state.apply_shot((1, 1).into()), HitType::Miss);
        assert_eq!(state, state_expected, "round 1 should not change state");

        // Round 2
        let state_expected = GameState {
            ships: vec![
                Ship::new(ShipClass::Carrier, (2, 3), Direction::Vertical),
                Ship::new(ShipClass::Battleship, (3, 1), Direction::Horizontal).with_hit_mask(0x02),
                Ship::new(ShipClass::Cruiser, (4, 7), Direction::Vertical),
                Ship::new(ShipClass::Submarine, (7, 5), Direction::Horizontal),
                Ship::new(ShipClass::Destroyer, (7, 7), Direction::Horizontal),
            ],
            pepper,
        };
        assert_eq!(state.apply_shot((4, 1).into()), HitType::Hit);
        assert_eq!(state, state_expected, "round 2 does not match expected");

        /* TODO Finish up this test
        // Duplicate hit results in no state change
        let params3 = RoundInput::new(state1, 4, 1);
        let result3 = result2.clone();
        assert_eq!(params3.process(), result3);

        let params4 = RoundInput::new(result3.state, 3, 1);
        let result4 = RoundOutput::new(
            GameState {
                ships: [
                    Ship::new(ShipClass::Carrier, (2, 3), Direction::Vertical),
                    Ship::new(ShipClass::Battleship, (3, 1), Direction::Horizontal),
                    Ship::new(ShipClass::Cruiser, (4, 7), Direction::Vertical),
                    Ship::new(ShipClass::Submarine, (7, 5), Direction::Horizontal),
                    Ship::new(ShipClass::Destroyer, (7, 7), Direction::Horizontal),
                ],
                pepper: salt,
            },
            HitType::Hit,
        );
        assert_eq!(params4.process(), result4);

        let params5 = RoundInput::new(result4.state, 6, 1);
        let result5 = RoundOutput::new(
            GameState {
                ships: [
                    Ship::new(ShipClass::Carrier, (2, 3), Direction::Vertical),
                    Ship::new(ShipClass::Battleship, (3, 1), Direction::Horizontal),
                    Ship::new(ShipClass::Cruiser, (4, 7), Direction::Vertical),
                    Ship::new(ShipClass::Submarine, (7, 5), Direction::Horizontal),
                    Ship::new(ShipClass::Destroyer, (7, 7), Direction::Horizontal),
                ],
                pepper: salt,
            },
            HitType::Hit,
        );
        assert_eq!(params5.process(), result5);

        let params6 = RoundInput::new(result5.state, 5, 1);
        let result6 = RoundOutput::new(
            GameState {
                ships: [
                    Ship::new(ShipClass::Carrier, (2, 3), Direction::Vertical),
                    Ship::new(ShipClass::Battleship, (3, 1), Direction::Horizontal),
                    Ship::new(ShipClass::Cruiser, (4, 7), Direction::Vertical),
                    Ship::new(ShipClass::Submarine, (7, 5), Direction::Horizontal),
                    Ship::new(ShipClass::Destroyer, (7, 7), Direction::Horizontal),
                ],
                pepper: salt,
            },
            HitType::Sunk(1),
        );
        assert_eq!(params6.process(), result6);
        */
    }
}
