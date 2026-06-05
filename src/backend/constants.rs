#![allow(unused)]
// The unused lint was disabled for this file
// because I want to keep some square constants that are not needed atm.

use crate::backend::types::bitboard::BitBoard;
use crate::backend::types::square::Square;

pub const STARTING_POS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub const SIDE_LENGTH: i8 = 8;
pub const SQUARES_AMOUNT: usize = 64;

// Pawn, Rook, Knight, Bishop, Queen, King
pub const PIECE_TYPE_COUNT: usize = 6;

// White, Black
pub const SIDES: usize = 2;

// Two bitboards for masking out the right and left side of the field.
pub const LEFT_SIDE_BB: BitBoard = BitBoard {
    value: 0x101010101010101,
};
pub const RIGHT_SIDE_BB: BitBoard = BitBoard {
    value: 0x8080808080808080,
};

// All Squares
pub const A1: Square = 0;
pub const B1: Square = 1;
pub const C1: Square = 2;
pub const D1: Square = 3;
pub const E1: Square = 4;
pub const F1: Square = 5;
pub const G1: Square = 6;
pub const H1: Square = 7;

pub const A2: Square = 8;
pub const B2: Square = 9;
pub const C2: Square = 10;
pub const D2: Square = 11;
pub const E2: Square = 12;
pub const F2: Square = 13;
pub const G2: Square = 14;
pub const H2: Square = 15;
pub const A7: Square = 48;

pub const B7: Square = 49;
pub const C7: Square = 50;
pub const D7: Square = 51;
pub const E7: Square = 52;
pub const F7: Square = 53;
pub const G7: Square = 54;
pub const H7: Square = 55;

pub const A8: Square = 56;
pub const B8: Square = 57;
pub const C8: Square = 58;
pub const D8: Square = 59;
pub const E8: Square = 60;
pub const F8: Square = 61;
pub const G8: Square = 62;
pub const H8: Square = 63;
