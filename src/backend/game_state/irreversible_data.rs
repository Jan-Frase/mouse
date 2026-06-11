use crate::backend::types::moove::CastleType;
use crate::backend::types::piece::{Piece, Side};
use crate::backend::types::square::Square;

/// The `IrreversibleData` struct stores data that is irreversible.
/// For example, this remembers what kind of piece was captured for `unmake_move()`.
#[derive(Debug, Clone)]
pub struct IrreversibleData {
    pub captured_piece: Option<Piece>,
    pub en_passant_square: Option<Square>,
    // TODO: squeeze this into a bitfield?
    pub white_long_castle_rights: bool,
    pub white_short_castle_rights: bool,
    pub black_long_castle_rights: bool,
    pub black_short_castle_rights: bool,
}

impl IrreversibleData {
    pub fn new() -> IrreversibleData {
        IrreversibleData {
            captured_piece: None,
            en_passant_square: None,
            white_long_castle_rights: false,
            white_short_castle_rights: false,
            black_long_castle_rights: false,
            black_short_castle_rights: false,
        }
    }

    pub fn new_with_castling_true() -> IrreversibleData {
        IrreversibleData {
            captured_piece: None,
            en_passant_square: None,
            white_long_castle_rights: true,
            white_short_castle_rights: true,
            black_long_castle_rights: true,
            black_short_castle_rights: true,
        }
    }

    pub fn new_from_previous_state(previous_state: &IrreversibleData) -> IrreversibleData {
        IrreversibleData {
            captured_piece: None,
            en_passant_square: None,
            white_long_castle_rights: previous_state.white_long_castle_rights,
            white_short_castle_rights: previous_state.white_short_castle_rights,
            black_long_castle_rights: previous_state.black_long_castle_rights,
            black_short_castle_rights: previous_state.black_short_castle_rights,
        }
    }

    pub fn get_long_castle_rights(&self, color: Side) -> bool {
        match color {
            Side::White => self.white_long_castle_rights,
            Side::Black => self.black_long_castle_rights,
        }
    }

    pub fn get_short_castle_rights(&self, color: Side) -> bool {
        match color {
            Side::White => self.white_short_castle_rights,
            Side::Black => self.black_short_castle_rights,
        }
    }

    pub fn get_castle_rights(&self, color: Side, castle_type: CastleType) -> bool {
        match castle_type {
            CastleType::Long => self.get_long_castle_rights(color),
            CastleType::Short => self.get_short_castle_rights(color),
        }
    }

    pub fn remove_long_castle_rights(&mut self, color: Side) {
        match color {
            Side::White => self.white_long_castle_rights = false,
            Side::Black => self.black_long_castle_rights = false,
        }
    }

    pub fn remove_short_castle_rights(&mut self, color: Side) {
        match color {
            Side::White => self.white_short_castle_rights = false,
            Side::Black => self.black_short_castle_rights = false,
        }
    }

    pub fn remove_castle_rights(&mut self, color: Side, castle_type: CastleType) {
        match castle_type {
            CastleType::Long => self.remove_long_castle_rights(color),
            CastleType::Short => self.remove_short_castle_rights(color),
        }
    }
}

impl Default for IrreversibleData {
    fn default() -> Self {
        Self::new()
    }
}
