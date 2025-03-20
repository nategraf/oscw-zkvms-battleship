// Copyright 2022 Risc0, Inc.
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ship {
    pub class: ShipClass,
    pub pos: Position,
    pub dir: Direction,
    pub hit_mask: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameState {
    pub ships: Vec<Ship>,
    /// Entropy added to the game state such that the commitment is hiding.
    pub pepper: [u8; 16],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RoundInput {
    pub state: GameState,
    pub shot: Position,
}

#[derive(Clone, Debug, Deserialize, Serialize, Hash)]
pub enum HitType {
    Miss,
    Hit,
    Sunk(ShipClass),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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
}

impl Distribution<GameState> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GameState {
        // Create a shuffled list of all positions on the board.
        let mut positions: Vec<Position> = (0..BOARD_SIZE)
            .zip(0..BOARD_SIZE)
            .map(|(x, y)| Position::new(x as u32, y as u32))
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

impl RoundInput {
    pub fn new(state: GameState, x: u32, y: u32) -> Self {
        RoundInput {
            state,
            shot: Position::new(x, y),
        }
    }

    pub fn process(&self) -> RoundOutput {
        let mut state = self.state.clone();
        let mut hit = HitType::Miss;
        for ship in state.ships.iter_mut() {
            hit = ship.shoot_at(self.shot);
            match hit {
                HitType::Hit | HitType::Sunk(_) => break,
                HitType::Miss => continue,
            }
        }
        RoundOutput { state, hit }
    }
}

impl RoundOutput {
    pub fn new(state: GameState, hit: HitType) -> Self {
        RoundOutput { state, hit }
    }
}

impl Position {
    pub fn new(x: u32, y: u32) -> Self {
        Position { x, y }
    }

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
    pub fn new(class: ShipClass, pos: Position, dir: Direction) -> Self {
        Ship {
            class,
            pos,
            dir,
            hit_mask: 0,
        }
    }

    pub fn shoot_at(&mut self, shot: Position) -> HitType {
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
            ships: [
                Ship::new(2, 3, Direction::Vertical),
                Ship::new(3, 1, Direction::Horizontal),
                Ship::new(4, 7, Direction::Vertical),
                Ship::new(7, 5, Direction::Horizontal),
                Ship::new(7, 7, Direction::Horizontal),
            ],
            pepper: 0xDEADBEEF,
        };

        assert!(state.check());
    }

    #[test]
    fn overlap() {
        let state = GameState {
            ships: [
                Ship::new(2, 3, Direction::Vertical),
                Ship::new(3, 1, Direction::Horizontal),
                Ship::new(2, 3, Direction::Vertical),
                Ship::new(7, 5, Direction::Horizontal),
                Ship::new(7, 7, Direction::Horizontal),
            ],
            pepper: 0xDEADBEEF,
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

        let salt = 0xDEADBEEF;
        let state1 = GameState {
            ships: [
                Ship::new(2, 3, Direction::Vertical),
                Ship::new(3, 1, Direction::Horizontal),
                Ship::new(4, 7, Direction::Vertical),
                Ship::new(7, 5, Direction::Horizontal),
                Ship::new(7, 7, Direction::Horizontal),
            ],
            pepper: salt,
        };

        let params1 = RoundInput::new(state1.clone(), 1, 1);
        let result1 = RoundOutput::new(state1.clone(), HitType::Miss);
        assert_eq!(params1.process(), result1);

        let params2 = RoundInput::new(state1.clone(), 4, 1);
        let result2 = RoundOutput::new(
            GameState {
                ships: [
                    Ship::new(2, 3, Direction::Vertical),
                    Ship::with_hit_mask(3, 1, Direction::Horizontal, 0x02),
                    Ship::new(4, 7, Direction::Vertical),
                    Ship::new(7, 5, Direction::Horizontal),
                    Ship::new(7, 7, Direction::Horizontal),
                ],
                pepper: salt,
            },
            HitType::Hit,
        );
        assert_eq!(params2.process(), result2);

        // Duplicate hit results in no state change
        let params3 = RoundInput::new(state1, 4, 1);
        let result3 = result2.clone();
        assert_eq!(params3.process(), result3);

        let params4 = RoundInput::new(result3.state, 3, 1);
        let result4 = RoundOutput::new(
            GameState {
                ships: [
                    Ship::new(2, 3, Direction::Vertical),
                    Ship::with_hit_mask(3, 1, Direction::Horizontal, 0x03),
                    Ship::new(4, 7, Direction::Vertical),
                    Ship::new(7, 5, Direction::Horizontal),
                    Ship::new(7, 7, Direction::Horizontal),
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
                    Ship::new(2, 3, Direction::Vertical),
                    Ship::with_hit_mask(3, 1, Direction::Horizontal, 0x0b),
                    Ship::new(4, 7, Direction::Vertical),
                    Ship::new(7, 5, Direction::Horizontal),
                    Ship::new(7, 7, Direction::Horizontal),
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
                    Ship::new(2, 3, Direction::Vertical),
                    Ship::with_hit_mask(3, 1, Direction::Horizontal, 0x0f),
                    Ship::new(4, 7, Direction::Vertical),
                    Ship::new(7, 5, Direction::Horizontal),
                    Ship::new(7, 7, Direction::Horizontal),
                ],
                pepper: salt,
            },
            HitType::Sunk(1),
        );
        assert_eq!(params6.process(), result6);
    }
}
