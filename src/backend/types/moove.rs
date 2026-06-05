use crate::backend::types::piece::{PROMOTABLE_PIECES, Piece};
use crate::backend::types::square::{Square, get_file, square_to_string, square_from_rank_and_file};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use crate::backend::types::piece::Piece::*;

#[derive(Copy, Clone)]
pub enum CastleType {
    Long,
    Short,
}

impl CastleType {
    pub fn get_all_types() -> [CastleType; 2] {
        [CastleType::Long, CastleType::Short]
    }
}

/// This encodes a single move. Sidenote: This is called Moove, since Move is a keyword in Rust...
/// It knows where a piece moved from and where it moved to.
/// Also stores to which piece a pawn promoted if one did at all.
///
/// Based on: https://github.com/official-stockfish/Stockfish/blob/master/src/types.h
///  It's stored in 16 bits
/// The first six are for the from index, the next six for the to index, leaving us with 4 bits remaining.
/// Two of those are used to encode the type of promotion piece. Either Rook, Knight, Bishop, or Queen
/// The next stores whether promotion has occurred
/// One bit and thus three possible data points free!
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Moove {
    bitfield: u16,
}

impl Moove {
    /// Creates a new `Move` instance with 'promotion_type' set to 0.
    pub const fn new(from: Square, to: Square) -> Moove {
        let mut result = from as u16 | ((to as u16) << 6);
        result |= 0;
        Moove { bitfield: result }
    }

    pub fn new_promotion(from: Square, to: Square, promotion_type: Piece) -> Moove {
        Moove {
            bitfield: from as u16 | ((to as u16) << 6) | (promotion_type as u16) << 12 | 1 << 14,
        }
    }

    pub fn get_from(&self) -> Square {
        let mask = 0b0000_0000_0011_1111u16;
        (self.bitfield & mask) as Square
    }

    pub fn get_to(&self) -> Square {
        let mask = 0b0000_1111_1100_0000u16;
        ((self.bitfield & mask) >> 6) as Square
    }

    pub fn get_promotion_type(&self) -> Option<Piece> {
        let promo_mask = 0b0100_0000_0000_0000u16;
        if (self.bitfield & promo_mask) == 0 {
            return None;
        }

        let type_mask = 0b0011_0000_0000_0000u16;
        let piece_index = (self.bitfield & type_mask) >> 12;

        Some(PROMOTABLE_PIECES[piece_index as usize])
    }

    /// This assumes that the moved piece is a pawn and only checks if the rank changed by 2.
    pub fn is_double_pawn_push(&self) -> bool {
        self.get_from().abs_diff(self.get_to()) == 16
    }

    /// This assumes that the moved piece is a king and only checks if the file changed by 2.
    pub fn is_castle(&self) -> bool {
        self.get_from().abs_diff(self.get_to()) == 2
    }

    pub fn get_castle_type(&self) -> CastleType {
        if get_file(self.get_to()) == 6 {
            CastleType::Short
        } else {
            CastleType::Long
        }
    }
}

impl From<&str> for Moove {
    fn from(value: &str) -> Self {
        Moove::moove_from_uci_notation(value)
    }
}

impl Moove {
    // -------------------
    // This is used during debugging and the uci interface.
    #[allow(unused)]
    fn square_from_uci_notation(uci_notation: &str) -> Square {
        let mut file = 0;
        let mut rank = 0;

        for char in uci_notation.chars() {
            match char {
                'a'..='h' => file = char.to_digit(36).unwrap() - 10,
                '1'..='8' => rank = char.to_digit(10).unwrap() - 1,
                _ => panic!("Invalid uci notation"),
            }
        }

        square_from_rank_and_file(rank as i8, file as i8)
    }

    // This is used during debugging and the uci interface.
    #[allow(unused)]
    pub fn moove_from_uci_notation(uci_notation: &str) -> Moove {
        let from = Moove::square_from_uci_notation(&uci_notation[0..2]);
        let to = Moove::square_from_uci_notation(&uci_notation[2..4]);

        let promotion_char = uci_notation.chars().nth(4);
        if let Some(char) = promotion_char {
            let promotion_type = match char {
                'r' => (Rook),
                'n' => (Knight),
                'b' => (Bishop),
                'q' => (Queen),
                _ => panic!("Invalid promotion type {:?}", uci_notation),
            };
            return Moove::new_promotion(from, to, promotion_type);
        };

        Moove::new(from, to)
    }
}



/// Converts a `Move` instance into an uci formatted string.
impl Display for Moove {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();

        result.push_str(&square_to_string(self.get_from()));
        result.push_str(&square_to_string(self.get_to()));
        result.push_str(match self.get_promotion_type() {
            None => "",
            Some(promotion_type) => match promotion_type {
                Piece::Rook => "r",
                Piece::Knight => "n",
                Piece::Bishop => "b",
                Piece::Queen => "q",
                _ => panic!("Invalid promotion type {:?}", promotion_type),
            },
        });

        write!(f, "{}", result)
    }
}

/// Implements ordering. Needed to sort them when comparing with perftree output.
/// This should only be called during debugging, not for performance-critical operations.
impl PartialOrd for Moove {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Moove {
    fn cmp(&self, other: &Self) -> Ordering {
        let own_from = self.get_from();
        let other_from = other.get_from();

        let own_to = self.get_to();
        let other_to = self.get_to();

        if own_from > other_from {
            return Ordering::Greater;
        } else if own_from < other_from {
            return Ordering::Less;
        }

        if own_to > other_to {
            return Ordering::Greater;
        } else if own_to < other_to {
            return Ordering::Less;
        }
        Ordering::Equal
    }
}
