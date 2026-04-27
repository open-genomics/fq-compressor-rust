use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

const INPUT_REPEAT_COUNT: usize = 2_048;
const INPUT_FIXTURE_NAME: &str = "benchmark-input.fastq";

static INPUT_BYTES: OnceLock<Vec<u8>> = OnceLock::new();

pub fn load_input_bytes() -> &'static [u8] {
    INPUT_BYTES
        .get_or_init(|| fs::read(bench_input_path()).unwrap())
        .as_slice()
}

fn bench_input_path() -> PathBuf {
    let path = bench_data_dir().join(INPUT_FIXTURE_NAME);
    if path.exists() {
        return path;
    }

    let seed = fs::read(test_data_dir().join("test_se.fastq")).unwrap();
    let mut expanded = Vec::with_capacity(seed.len() * INPUT_REPEAT_COUNT);
    for _ in 0..INPUT_REPEAT_COUNT {
        expanded.extend_from_slice(&seed);
    }
    fs::write(&path, expanded).unwrap();
    path
}

fn bench_data_dir() -> PathBuf {
    let dir = repo_root().join("target").join("bench-data");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn test_data_dir() -> PathBuf {
    repo_root().join("tests").join("data")
}
