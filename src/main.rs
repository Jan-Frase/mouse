use crate::backend::game_state::fen_parser::moove_from_uci_notation;
use crate::backend::game_state::state::State;
use crate::backend::perft::perft;
use std::env;
use std::env::Args;
use crate::backend::movegen::move_gen::MoveGenerator;

mod backend;

fn main() {
    let args = env::args();
    run_perftree_debug(args);
}

// --------------------------------------------- //
// PERFTREE DEBUGGING
// https://github.com/agausmann/perftree
// --------------------------------------------- //

pub fn run_perftree_debug(mut input: Args) {
    // Remove the first useless input.
    input.next();

    let depth = input.next().unwrap();
    let depth = depth.parse::<i32>().unwrap();

    let fen = &input.next().unwrap();
    let mut state = State::new_from_fen(fen);

    for mooves in input {
        // Code golfing
        mooves
            .split_whitespace()
            .map(moove_from_uci_notation)
            .for_each(|moove| {
                state = state.make_move(moove);
            });
    }

    root_debug_perft(&mut state, depth as u8);
}

pub fn root_debug_perft(root_state: &mut State, depth: u8) -> u64 {
    // Total nodes searched.
    let mut nodes = 0;

    // Generate all root moves.
    let mut move_generator = MoveGenerator::new();
    let mut moves = move_generator.generate_moves(root_state);
    // Sort them in the same way as perftree does
    moves.sort();

    // Catch trivial depth 1 case
    if depth == 1 {
        nodes = moves.len() as u64;

        for moove in moves {
            println!("{} {:?}", moove, 1);
        }

        println!();
        println!("{:?}", nodes);
        return nodes;
    }

    for chess_move in moves {
        let mut state = root_state.make_move(chess_move);

        // Recursively calculate nodes for this position.
        let nodes_for_this_position = perft(&mut state, depth - 1);
        nodes += nodes_for_this_position;
        // print info for https://github.com/agausmann/perftree
        println!("{} {:?}", chess_move, nodes_for_this_position);
    }

    println!();
    println!("{:?}", nodes);
    nodes
}
