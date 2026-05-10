use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use mouse::backend::perft::perft;
use mouse::{moves, State};
use perft_fixtures::perft_fixtures::FAST_PERFT;

pub fn criterion_perft(c: &mut Criterion) {
    for perft_set_up in FAST_PERFT {
        let fen_string = perft_set_up.perft_setup.fen;
        let name = perft_set_up.perft_setup.name.to_owned()
            + ", depth: "
            + perft_set_up.depth.to_string().as_str();
        let depth = perft_set_up.depth;
        let expected_nodes = perft_set_up.expected_nodes;

        let mut state = State::new_from_fen(fen_string);

        run_criterion_perft(c, name, depth, expected_nodes, &mut state);
    }
}

fn run_criterion_perft(
    c: &mut Criterion,
    name: String,
    depth: u8,
    expected_nodes: u64,
    state: &mut State,
) {
    let mut group = c.benchmark_group(&*name);
    // group.sample_size(10);
    // group.sampling_mode(Flat);
    // group.warm_up_time(std::time::Duration::from_millis(1000));
    group.throughput(Throughput::Elements(expected_nodes));
    group.bench_function(&*name, |b| {
        b.iter(|| perft(std::hint::black_box(state), std::hint::black_box(depth)))
    });
    group.finish();
}

pub fn criterion_make_unmake_move(c: &mut Criterion) {
    let mut state =
        State::new_from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQ - 0 1");
    let moves = moves(&mut state);

    let mut group = c.benchmark_group("General make unmake");
    group.throughput(Throughput::Elements(moves.len() as u64));
    group.bench_function("General make unmake", |b| {
        b.iter(|| {
            for moove in &moves[0..moves.len()] {
                let _ = state.make_move(std::hint::black_box(*moove));
            }
        })
    });
    group.finish();
}

pub fn criterion_move_gen(c: &mut Criterion) {
    let mut state =
        State::new_from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let expected_moves = 48;

    let mut group = c.benchmark_group("Move gen");
    group.throughput(Throughput::Elements(expected_moves));
    group.bench_function("Move gen", |b| {
        b.iter(|| {
            let _ = moves(std::hint::black_box(&mut state));
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    criterion_perft,
    criterion_make_unmake_move,
    criterion_move_gen
);
criterion_main!(benches);
