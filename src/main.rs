use crate::backend::game_state::state::State;
use crate::backend::movegen::move_gen::moves;
use crate::backend::perft::perft;
use std::env::Args;
use crate::backend::types::moove::Moove;

mod backend;

fn main() {
    // let mut state = State::new_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    // root_debug_perft(&mut state, 7);

    // let args = env::args();
    // run_perftree_debug(args);

    let mut state = State::new_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let _ = perft(&mut state, 7);

    let mut state =
        State::new_from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let _ = perft(&mut state, 6);
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
            .map(Moove::moove_from_uci_notation)
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
    let mut moves = moves(root_state);
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
