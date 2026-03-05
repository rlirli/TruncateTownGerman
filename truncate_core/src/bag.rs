use oorandom::Rand32;
use std::fmt;

use crate::rules;

/*
INFO: Letter distributions in Truncate's dict (German, 35939 words)

   | <= 3   | <= 5   | <= 7   | overall
a: | 11.01  |  9.07  |  7.53  |  6.29
b: |  4.19  |  2.95  |  2.72  |  2.40
c: |  1.42  |  1.89  |  2.15  |  3.26
d: |  4.12  |  2.69  |  2.28  |  2.06
e: |  8.89  | 14.07  | 16.88  | 17.82
f: |  1.67  |  2.23  |  2.20  |  2.06
g: |  3.48  |  3.00  |  3.41  |  3.80
h: |  4.25  |  3.85  |  3.65  |  4.70
i: |  6.83  |  5.96  |  5.66  |  6.22
j: |  1.22  |  0.60  |  0.31  |  0.15
k: |  2.38  |  3.00  |  2.70  |  2.08
l: |  3.35  |  5.70  |  5.24  |  4.57
m: |  4.44  |  3.32  |  2.81  |  2.32
n: |  4.83  |  6.16  |  7.87  |  8.63
o: |  7.15  |  4.96  |  3.75  |  2.86
p: |  3.28  |  2.25  |  1.98  |  1.41
q: |  0.00  |  0.09  |  0.07  |  0.05
r: |  5.15  |  6.44  |  7.26  |  7.66
s: |  5.22  |  6.37  |  6.53  |  6.64
t: |  4.96  |  7.00  |  7.54  |  7.23
u: |  6.44  |  4.49  |  4.00  |  4.25
v: |  0.84  |  0.57  |  0.60  |  0.99
w: |  2.32  |  1.42  |  1.20  |  1.08
x: |  0.84  |  0.28  |  0.15  |  0.08
y: |  0.64  |  0.47  |  0.28  |  0.14
z: |  1.09  |  1.19  |  1.23  |  1.23

*/

const TILE_GENERATIONS: [[usize; 26]; 2] = [
    [
        6,  // a
        2,  // b
        3,  // c
        2,  // d
        18, // e
        2,  // f
        4,  // g
        5,  // h
        6,  // i
        1,  // j
        2,  // k
        4,  // l
        2,  // m
        9,  // n
        3,  // o
        1,  // p
        1,  // q
        8,  // r
        7,  // s
        7,  // t
        4,  // u
        1,  // v
        1,  // w
        1,  // x
        1,  // y
        1,  // z
    ],
    [
        8,  // a
        3,  // b
        2,  // c
        2,  // d
        17, // e
        2,  // f
        3,  // g
        4,  // h
        6,  // i
        1,  // j
        3,  // k
        4,  // l
        3,  // m
        8,  // n
        4,  // o
        2,  // p
        1,  // q
        7,  // r
        7,  // s
        8,  // t
        4,  // u
        1,  // v
        1,  // w
        1,  // x
        1,  // y
        1,  // z
    ],
];

#[derive(Debug, Clone)]
pub struct TileBag {
    bag: Vec<char>,
    rng: Rand32,
    letter_distribution: Option<[usize; 26]>,
}

impl TileBag {
    pub fn generation(gen: u32, seed: Option<u64>) -> Self {
        TileBag::custom(
            TILE_GENERATIONS
                .get(gen as usize)
                .expect("Tilebag generation should exist")
                .clone(),
            seed,
        )
    }

    pub fn latest(seed: Option<u64>) -> (u32, Self) {
        assert!(!TILE_GENERATIONS.is_empty());
        let generation = (TILE_GENERATIONS.len() - 1) as u32;
        (generation, TileBag::generation(generation as u32, seed))
    }

    pub fn custom(letter_distribution: [usize; 26], seed: Option<u64>) -> Self {
        let mut tile_bag = TileBag {
            bag: Vec::new(),
            rng: Rand32::new(seed.unwrap_or_else(|| {
                instant::SystemTime::now()
                    .duration_since(instant::SystemTime::UNIX_EPOCH)
                    .expect("Please don't play Truncate earlier than 1970")
                    .as_secs()
            })),
            letter_distribution: Some(letter_distribution),
        };
        tile_bag.fill();
        tile_bag
    }

    pub fn explicit(tiles: Vec<char>, seed: Option<u64>) -> Self {
        TileBag {
            bag: tiles,
            rng: Rand32::new(seed.unwrap_or_else(|| {
                instant::SystemTime::now()
                    .duration_since(instant::SystemTime::UNIX_EPOCH)
                    .expect("Please don't play Truncate earlier than 1970")
                    .as_secs()
            })),
            letter_distribution: None,
        }
    }

    pub fn draw_tile(&mut self) -> char {
        if self.bag.is_empty() {
            self.fill();
        }
        let index = self.rng.rand_range(0..self.bag.len() as u32);
        self.bag.swap_remove(index as usize)
    }

    // TODO: this doesn't stop us from returning tiles that weren't originally in the bag
    pub fn return_tile(&mut self, c: char) {
        self.bag.push(c);
    }

    fn fill(&mut self) {
        if let Some(letter_distribution) = self.letter_distribution {
            self.bag.extend(
                letter_distribution
                    .iter()
                    .enumerate()
                    .flat_map(|(letter, count)| [((letter as u8) + 65) as char].repeat(*count)),
            );
        }
    }
}

impl PartialEq for TileBag {
    fn eq(&self, rhs: &Self) -> bool {
        self.bag == rhs.bag && self.letter_distribution == rhs.letter_distribution
    }
}

impl fmt::Display for TileBag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Letters in the bag:\n{:?}", self.bag)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn refills() {
        let mut bag = a_b_bag();
        assert_eq!(bag.to_string(), "Letters in the bag:\n['A', 'B']");
        let drawn = (0..10).map(|_| bag.draw_tile());
        assert_eq!(drawn.filter(|&x| x == 'A').count(), 5);
    }

    // Util functions
    pub fn a_b_bag() -> TileBag {
        let mut dist = [0; 26];
        dist[0] = 1; // There is 1 A and
        dist[1] = 1; // 1 B in the bag
        TileBag::custom(dist, Some(12345))
    }

    pub fn trivial_bag() -> TileBag {
        let mut dist = [0; 26];
        dist[0] = 1;
        TileBag::custom(dist, Some(12345))
    }
}
