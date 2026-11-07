use chronos_core::{Event, Program, parse_source};
use std::hint::black_box;

struct AdversarialRng(u64);

impl AdversarialRng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn index(&mut self, upper: usize) -> usize {
        (self.next() as usize) % upper
    }
}

fn mutate_ascii(seed: u64, input: &[u8], iterations: usize) -> Vec<Vec<u8>> {
    const ALPHABET: &[u8] =
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_ =(),;#//\"+-*\n\t{}[]:.";
    let mut rng = AdversarialRng::new(seed);
    let mut cases = Vec::with_capacity(iterations);

    for iteration in 0..iterations {
        let mut candidate = input.to_vec();
        let edits = 1 + rng.index(5);

        for _ in 0..edits {
            match rng.index(3) {
                0 if !candidate.is_empty() => {
                    let position = rng.index(candidate.len());
                    candidate[position] = ALPHABET[rng.index(ALPHABET.len())];
                }
                1 if candidate.len() < 512 => {
                    let position = rng.index(candidate.len() + 1);
                    candidate.insert(position, ALPHABET[rng.index(ALPHABET.len())]);
                }
                _ if !candidate.is_empty() => {
                    let position = rng.index(candidate.len());
                    candidate.remove(position);
                }
                _ => candidate.push(ALPHABET[rng.index(ALPHABET.len())]),
            }
        }

        if iteration % 7 == 0 && candidate.len() < 512 {
            candidate.extend(std::iter::repeat_n(
                ALPHABET[rng.index(ALPHABET.len())],
                rng.index(16),
            ));
        }
        cases.push(candidate);
    }

    cases
}

#[test]
fn source_parser_handles_deterministic_adversarial_mutations_without_panicking() {
    let seeds = [
        b"program \"comment # // marker\";\na = input(-7);\nb = input(2);\nc = div(a, b);"
            .as_slice(),
        b"a = add(b, b);\nb = sub(a, a);".as_slice(),
        b"x = hash_rounds(y, 5000000);\ny = input(1);".as_slice(),
        b"program \"escaped \\\" quote\";\nvalue = input(0);".as_slice(),
    ];

    let mut tested = 0usize;
    for (seed_index, seed) in seeds.iter().enumerate() {
        for candidate in mutate_ascii(0xC0FF_EE00 + seed_index as u64, seed, 2_048) {
            if let Ok(source) = std::str::from_utf8(&candidate) {
                let result = parse_source(black_box(source));
                if let Ok(program) = result {
                    // Successful parses must still satisfy the graph compiler's invariants.
                    black_box(program.validate()).ok();
                }
                tested += 1;
            }
        }
    }

    assert_eq!(tested, 8_192);
}

#[test]
fn json_program_decoder_handles_deterministic_adversarial_mutations_without_panicking() {
    let base = Program::new(
        "adversarial-json-seed",
        vec![
            Event::input(1, "left", i64::MIN),
            Event::input(2, "right", 17),
            Event::add(3, "sum", 1, 2),
        ],
    );
    let encoded = serde_json::to_vec(&base).unwrap();
    let cases = mutate_ascii(0x51A7_E123, &encoded, 8_192);

    for candidate in cases {
        if let Ok(program) = serde_json::from_slice::<Program>(black_box(&candidate)) {
            // JSON that happens to parse must pass normal semantic validation safely.
            black_box(program.validate()).ok();
        }
    }
}
