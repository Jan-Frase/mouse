use crate::backend::constants::{A1, A8, H1, H8};
use crate::backend::game_state::bb_manager::BBManager;
use crate::backend::game_state::fen_parser::parse_fen;
use crate::backend::game_state::irreversible_data::IrreversibleData;
use crate::backend::types::bitboard::BitBoard;
use crate::backend::types::moove::{CastleType, Moove};
use crate::backend::types::piece::Piece::{King, Pawn, Rook};
use crate::backend::types::piece::{Piece, Side};
use crate::backend::types::square::{Square, back_by_one};

const ROOK_SWAP_WHITE_LONG_CASTLE_BB: BitBoard = BitBoard { value: 0x9 };
const ROOK_SWAP_WHITE_SHORT_CASTLE_BB: BitBoard = BitBoard { value: 0xa0 };
const ROOK_SWAP_BLACK_LONG_CASTLE_BB: BitBoard = BitBoard {
    value: 0x900000000000000,
};
const ROOK_SWAP_BLACK_SHORT_CASTLE_BB: BitBoard = BitBoard {
    value: 0xa000000000000000,
};

#[derive(Debug, Clone)]
pub struct State {
    pub bb_mngr: BBManager,
    pub irreversible_data: IrreversibleData,
    pub active_side: Side,
    pub half_move_clock: u16,
}

impl State {
    /// Creates a new `GameState` instance with default values.
    /// This is not turned into a `default` as many constructors in this program need to be const.
    /// Those can't be `defaulted and I would rather keep it consistent.
    #[allow(clippy::new_without_default)]
    pub fn new() -> State {
        State {
            bb_mngr: BBManager::new(),
            active_side: Side::White,
            irreversible_data: IrreversibleData::new_with_castling_true(),
            half_move_clock: 0,
        }
    }

    /// Creates a new `GameState` instance based on the fen string.
    pub fn new_from_fen(fen_string: &str) -> State {
        let mut bb_manager = BBManager::new();
        let mut active_color = Side::White;
        let mut irreversible_data = IrreversibleData::new();
        let mut half_move_clock = 0;

        parse_fen(
            fen_string,
            &mut bb_manager,
            &mut active_color,
            &mut irreversible_data,
            &mut half_move_clock,
        );

        State {
            bb_mngr: bb_manager,
            active_side: active_color,
            irreversible_data,
            half_move_clock,
        }
    }

    /// Executes a move.
    ///
    /// # Arguments
    ///
    /// * `chess_move` - A `Moove` object representing the move to be made.
    pub fn make_move(&self, moove: Moove) -> State {
        let mut next_state = self.clone();
        // The new irreversible data.
        let mut next_ir_data = IrreversibleData::new_from_previous_state(&self.irreversible_data);

        // Get the type of moved piece.
        let mut moved_piece = self.bb_mngr.get_piece_at_square(moove.get_from()).unwrap();

        // Usually the square something was captured on (if something was captured at all) is the square we moved to...
        let mut capture_square = moove.get_to();
        if moved_piece == Pawn {
            // ... unless this is an en passant capture, we then need to update the capture square.
            next_state.make_move_ep_capture(moove, &mut capture_square);
            // Check if a double pawn push was played and store the en passant file
            next_state.make_move_double_pawn_push(moove, &mut next_ir_data);
        }

        // If something was captured, remove the piece and update irreversible data.
        next_state.make_move_capture(&mut next_ir_data, capture_square);

        // Clear the square that the piece was moved from.
        next_state
            .bb_mngr
            .clear_square(moove.get_from(), moved_piece, self.active_side);

        // Update the moved piece bb if it was a pawn promotion
        match moove.get_promotion_type() {
            None => {}
            Some(promotion_type) => {
                moved_piece = promotion_type;
            }
        }
        // Fill the square it moved to.
        next_state
            .bb_mngr
            .fill_square(moove.get_to(), moved_piece, self.active_side);

        // Some special king handling
        if moved_piece == King {
            next_state.make_move_king(moove, &mut next_ir_data);
        }

        next_state.make_move_castling_rights_on_rook_move_or_capture(
            &mut next_ir_data,
            moved_piece,
            moove.get_from(),
            self.active_side,
        );

        // Take care of some basics.
        next_state.active_side = self.active_side.oppo();
        next_state.irreversible_data = next_ir_data;
        next_state
    }

    fn make_move_ep_capture(&mut self, moove: Moove, capture_square: &mut Square) {
        let ep_square = self.irreversible_data.en_passant_square;

        // if an en passant square exists
        if let Some(ep_square) = ep_square
            // and if we moved to the ep_square
            && ep_square == moove.get_to()
        {
            // update the captured square to the ep_square - offset
            *capture_square = back_by_one(moove.get_to(), self.active_side);
        }
    }

    fn make_move_capture(
        &mut self,
        irreversible_data: &mut IrreversibleData,
        capture_square: Square,
    ) {
        // Get the type of the captured piece if it exists.
        let captured_piece = self.bb_mngr.get_piece_at_square(capture_square);
        // Clear the square on the captured piece's bitboard if it exists.
        if let Some(captured_piece) = captured_piece {
            // Store the captured piece type in the irreversible data.
            irreversible_data.captured_piece = Some(captured_piece);
            // Remove the captured piece from its bitboard.
            self.bb_mngr
                .clear_square(capture_square, captured_piece, self.active_side.oppo());

            // Remove castling rights if the captured piece was a rook on its starting square
            self.make_move_castling_rights_on_rook_move_or_capture(
                irreversible_data,
                captured_piece,
                capture_square,
                self.active_side.oppo(),
            )
        }
    }

    fn make_move_double_pawn_push(
        &mut self,
        moove: Moove,
        irreversible_data: &mut IrreversibleData,
    ) {
        if moove.is_double_pawn_push() {
            // the pawn starting square and one forward
            let ep_square = back_by_one(moove.get_to(), self.active_side);

            irreversible_data.en_passant_square = Some(ep_square);
        }
    }

    fn make_move_king(&mut self, moove: Moove, irreversible_data: &mut IrreversibleData) {
        // If the king moved we can't castle anymore
        irreversible_data.remove_long_castle_rights(self.active_side);
        irreversible_data.remove_short_castle_rights(self.active_side);

        // If we castled, we need to move the rook
        if moove.is_castle() {
            let rook_bb = self.bb_mngr.get_piece_bb_mut(Rook);
            let rook_swap_bb = Self::get_rook_swap_bb(moove.get_castle_type(), self.active_side);
            *rook_bb ^= rook_swap_bb;
            let friendly_bb = self.bb_mngr.get_side_bb_mut(self.active_side);
            *friendly_bb ^= rook_swap_bb;
        }
    }

    fn get_rook_swap_bb(castle_type: CastleType, active_color: Side) -> BitBoard {
        match castle_type {
            CastleType::Long => match active_color {
                Side::White => ROOK_SWAP_WHITE_LONG_CASTLE_BB,
                Side::Black => ROOK_SWAP_BLACK_LONG_CASTLE_BB,
            },
            CastleType::Short => match active_color {
                Side::White => ROOK_SWAP_WHITE_SHORT_CASTLE_BB,
                Side::Black => ROOK_SWAP_BLACK_SHORT_CASTLE_BB,
            },
        }
    }

    fn make_move_castling_rights_on_rook_move_or_capture(
        &mut self,
        irreversible_data: &mut IrreversibleData,
        piece_type: Piece,
        relevant_square: Square,
        relevant_side: Side,
    ) {
        if piece_type == Rook {
            for castling_type in CastleType::get_all_types() {
                let starting_square = Self::get_rook_starting_square(castling_type, relevant_side);
                if relevant_square == starting_square {
                    irreversible_data.remove_castle_rights(relevant_side, castling_type);
                }
            }
        }
    }

    fn get_rook_starting_square(castle_type: CastleType, color: Side) -> Square {
        match castle_type {
            CastleType::Long => match color {
                Side::White => A1,
                Side::Black => A8,
            },
            CastleType::Short => match color {
                Side::White => H1,
                Side::Black => H8,
            },
        }
    }
}
