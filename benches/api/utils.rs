use h3o::CellIndex;
use std::path::PathBuf;

pub fn load_dataset(name: &str) -> Vec<CellIndex> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let filepath = format!("dataset/{name}.cht");
    path.push(filepath);

    let bytes = std::fs::read(path).expect("read test data");
    h3o_zip::decompress(&bytes)
        .collect::<Result<_, _>>()
        .expect("unpack test data")
}
