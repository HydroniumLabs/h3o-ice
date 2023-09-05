use h3o::{CellIndex, Resolution};

// Max key size, in bytes (base cell + 15 children).
const SIZE: usize = 16;

/// A decomposed version of an H3 cell index.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Key([u8; SIZE]);

impl Key {
    #[allow(clippy::cast_possible_truncation)] // len is in [0; 15].
    const fn len(self) -> u8 {
        (15 - (u128::from_be_bytes(self.0).trailing_ones() / 8)) as u8
    }
}

impl From<CellIndex> for Key {
    fn from(value: CellIndex) -> Self {
        let mut key = [0xff; SIZE];
        key[0] = value.base_cell().into();
        // Store the size in the upper bits of the last cell.
        for res in Resolution::range(Resolution::One, value.resolution()) {
            key[res as usize] =
                value.direction_at(res).expect("resolution in range").into();
        }

        Self(key)
    }
}

impl AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        &self.0[..=usize::from(self.len())]
    }
}

impl From<Key> for CellIndex {
    #[allow(clippy::cast_possible_truncation)] // resolution is in [0; 15].
    fn from(value: Key) -> Self {
        let res = value.len();
        let key = value.0;

        // Default cell index (resolution 0, base cell 0).
        let mut index = 0x8001fffffffffff;
        // Resolution bit offset: 52.
        index |= u64::from(res) << 52;
        // Base cell bit offset: 45
        index |= u64::from(key[0]) << 45;

        for (i, direction) in key[1..=usize::from(res)].iter().enumerate() {
            let direction = u64::from(*direction);
            // +1 since we skip the first cell (base cell).
            let resolution = (i + 1) as u8;
            // Max res: 15, direction bit width: 3
            let offset = (15 - resolution) * 3;
            index = (index & !(0b111 << offset)) | (direction << offset);
        }

        Self::try_from(index).expect("valid cell index")
    }
}

impl From<&[u8]> for Key {
    fn from(value: &[u8]) -> Self {
        let mut key = [0xff; SIZE];
        let len = std::cmp::min(SIZE, value.len());
        key[..len].copy_from_slice(&value[..len]);
        Self(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from() {
        let index = CellIndex::try_from(0x8f2a1072b598641).expect("valid cell");
        let key = Key::from(index);
        assert_eq!(index, CellIndex::from(key));
    }
}
