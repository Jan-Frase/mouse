use crate::backend::game_state::bb_manager::BBManager;
use crate::backend::game_state::irreversible_data::IrreversibleData;
use crate::backend::types::piece::Piece::{Bishop, King, Knight, Pawn, Queen, Rook};
use crate::backend::types::piece::Side::{Black, White};
use crate::backend::types::piece::{Piece, Side};
use crate::backend::types::square::Square;
use crate::backend::types::square::square_from_rank_and_file;

/// Parses a FEN (Forsyth-Edwards Notation) string and updates the corresponding game game_state.
/// https://www.chessprogramming.org/Forsyth-Edwards_Notation
///
/// # Arguments
///
/// * `fen_string` - A string slice containing the FEN representation of the chess game.
/// * `bit_board_manager` - A mutable reference to a `BitBoardManager` to update the board positions.
/// * `active_color` - A mutable reference to a `PieceColor` to set the active player.
/// * `irreversible_data` - A mutable reference to `IrreversibleData`.
/// * `half_move_clock` - A mutable reference to a `u16` to update the current half-move clock count.
pub fn parse_fen(
    fen_string: &str,
    bit_board_manager: &mut BBManager,
    active_color: &mut Side,
    irreversible_data: &mut IrreversibleData,
    half_move_clock: &mut u16,
) {
    let fen_string = fen_string.split_whitespace().collect::<Vec<&str>>();

    let positions_string = fen_string[0];
    parse_position(bit_board_manager, positions_string);

    let active_color_string = fen_string[1];
    parse_active_color(active_color, active_color_string);

    let castling_rights_string = fen_string[2];
    parse_castling_rights(irreversible_data, castling_rights_string);

    let en_passant_file_string = fen_string[3];
    parse_en_passant(irreversible_data, en_passant_file_string);

    let half_move_clock_string = fen_string[4];
    *half_move_clock = half_move_clock_string.parse::<u16>().unwrap();

    let _full_move_number_string = fen_string[5];
    // I don't store this data as it isn't used for anything.
}

fn parse_en_passant(irreversible_data: &mut IrreversibleData, en_passant_file_string: &str) {
    if en_passant_file_string == "-" {
        irreversible_data.en_passant_square = None;
        return;
    }

    let mut file = 0;
    let mut rank = 0;
    for char in en_passant_file_string.chars() {
        match char {
            'a'..='h' => {
                file = char.to_digit(36).unwrap() as i8 - 10;
            }
            '3' | '6' => {
                rank = char.to_digit(10).unwrap() as i8 - 1;
            }
            _ => panic!("Invalid character in FEN string"),
        }
    }
    irreversible_data.en_passant_square = Some(square_from_rank_and_file(rank, file));
}

fn parse_castling_rights(irreversible_data: &mut IrreversibleData, castling_rights_string: &str) {
    for char in castling_rights_string.chars() {
        match char {
            '-' => {
                irreversible_data.white_long_castle_rights = false;
                irreversible_data.white_short_castle_rights = false;
                irreversible_data.black_long_castle_rights = false;
                irreversible_data.black_short_castle_rights = false;
            }
            'K' => {
                irreversible_data.white_short_castle_rights = true;
            }
            'k' => {
                irreversible_data.black_short_castle_rights = true;
            }
            'Q' => {
                irreversible_data.white_long_castle_rights = true;
            }
            'q' => {
                irreversible_data.black_long_castle_rights = true;
            }
            _ => panic!("Invalid character in FEN string"),
        }
    }
}

fn parse_active_color(active_color: &mut Side, active_color_string: &str) {
    match active_color_string {
        "w" => *active_color = Side::White,
        "b" => *active_color = Side::Black,
        _ => {
            panic!("Invalid character in FEN string");
        }
    }
}

fn parse_position(bit_board_manager: &mut BBManager, positions_string: &str) {
    let mut file = 0;
    let mut rank = 7;
    for char in positions_string.chars() {
        match char {
            '1'..='8' => {
                file += char.to_digit(10).unwrap() as i8;
            }
            '/' => {
                file = 0;
                rank -= 1;
            }
            'P' => {
                fill_square(
                    bit_board_manager,
                    Pawn,
                    White,
                    square_from_rank_and_file(rank, file),
                );
                file += 1;
            }
            'p' => {
                fill_square(
                    bit_board_manager,
                    Pawn,
                    Black,
                    square_from_rank_and_file(rank, file),
                );
                file += 1;
            }
            'R' => {
                fill_square(
                    bit_board_manager,
                    Rook,
                    White,
                    square_from_rank_and_file(rank, file),
                );
                file += 1;
            }
            'r' => {
                fill_square(
                    bit_board_manager,
                    Rook,
                    Black,
                    square_from_rank_and_file(rank, file),
                );
                file += 1;
            }
            'N' => {
                fill_square(
                    bit_board_manager,
                    Knight,
                    White,
                    square_from_rank_and_file(rank, file),
                );
                file += 1;
            }
            'n' => {
                fill_square(
                    bit_board_manager,
                    Knight,
                    Black,
                    square_from_rank_and_file(rank, file),
                );
                file += 1;
            }
            'B' => {
                fill_square(
                    bit_board_manager,
                    Bishop,
                    White,
                    square_from_rank_and_file(rank, file),
                );
                file += 1;
            }
            'b' => {
                fill_square(
                    bit_board_manager,
                    Bishop,
                    Black,
                    square_from_rank_and_file(rank, file),
                );
                file += 1;
            }
            'Q' => {
                fill_square(
                    bit_board_manager,
                    Queen,
                    White,
                    square_from_rank_and_file(rank, file),
                );
                file += 1;
            }
            'q' => {
                fill_square(
                    bit_board_manager,
                    Queen,
                    Black,
                    square_from_rank_and_file(rank, file),
                );
                file += 1;
            }
            'K' => {
                fill_square(
                    bit_board_manager,
                    King,
                    White,
                    square_from_rank_and_file(rank, file),
                );
                file += 1;
            }
            'k' => {
                fill_square(
                    bit_board_manager,
                    King,
                    Black,
                    square_from_rank_and_file(rank, file),
                );
                file += 1;
            }
            _ => {
                panic!("Invalid character in FEN string");
            }
        }
    }

    fn fill_square(bb_manager: &mut BBManager, piece_type: Piece, side: Side, square: Square) {
        bb_manager.get_piece_bb_mut(piece_type).fill_square(square);
        bb_manager
            .get_side_bb_mut(side)
            .fill_square(square);
    }
}
