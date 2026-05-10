use iai::black_box;
use mouse::State;
use mouse::backend::perft::perft;
use perft_fixtures::perft_fixtures::{FAST_PERFT, PerftFixture};

pub fn iai_perft(perft_fixture: &PerftFixture) {
    let fen_string = perft_fixture.perft_setup.fen;
    let _ = perft_fixture.perft_setup.name.to_owned()
        + ", depth: "
        + perft_fixture.depth.to_string().as_str();
    let depth = perft_fixture.depth;
    let _ = perft_fixture.expected_nodes;

    let mut state = State::new_from_fen(fen_string);

    let _ = perft(black_box(&mut state), black_box(depth));
}

pub fn starting_perft() {
    iai_perft(&FAST_PERFT[0]);
}

pub fn position_2_perft() {
    iai_perft(&FAST_PERFT[1]);
}

pub fn position_3_perft() {
    iai_perft(&FAST_PERFT[2]);
}

pub fn position_4_perft() {
    iai_perft(&FAST_PERFT[3]);
}

pub fn position_5_perft() {
    iai_perft(&FAST_PERFT[4]);
}

pub fn position_6_perft() {
    iai_perft(&FAST_PERFT[5]);
}

iai::main!(
    starting_perft,
    position_2_perft,
    position_3_perft,
    position_4_perft,
    position_5_perft,
    position_6_perft
);
