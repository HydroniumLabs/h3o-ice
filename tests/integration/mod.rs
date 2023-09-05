mod map;
mod set;

#[macro_export]
macro_rules! cell_index {
    ($x:expr) => {
        h3o::CellIndex::try_from($x).expect("invalid cell")
    };
}
