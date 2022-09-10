/// The side of a coin. This is the result type of [`coinflip`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Side {
    Heads = 0,
    Tails = 1,
}

impl Side {
    pub fn is_heads(&self) -> bool {
        match self {
            Side::Heads => true,
            Side::Tails => false,
        }
    }

    pub fn is_tails(&self) -> bool {
        !self.is_heads()
    }
}

/// Takes a randomness and returns the result of a coinflip (heads or tails)
pub fn coinflip(randomness: [u8; 32]) -> Side {
    if randomness[0] % 2 == 0 {
        Side::Heads
    } else {
        Side::Tails
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_is_heads_and_is_tails_works() {
        assert!(Side::Heads.is_heads());
        assert!(!Side::Heads.is_tails());

        assert!(Side::Tails.is_tails());
        assert!(!Side::Tails.is_heads());
    }

    #[test]
    fn coinflip_works() {
        let result = coinflip([
            88, 85, 86, 91, 61, 64, 60, 71, 234, 24, 246, 200, 35, 73, 38, 187, 54, 59, 96, 9, 237,
            27, 215, 103, 148, 230, 28, 48, 51, 114, 203, 219,
        ]);
        assert_eq!(result, Side::Heads);

        let result = coinflip([
            207, 251, 10, 105, 100, 223, 244, 6, 207, 231, 253, 206, 157, 68, 143, 184, 209, 222,
            70, 249, 114, 160, 213, 73, 147, 94, 136, 191, 94, 98, 99, 170,
        ]);
        assert_eq!(result, Side::Tails);

        let result = coinflip([
            43, 140, 160, 0, 187, 41, 212, 6, 218, 53, 58, 198, 80, 209, 171, 239, 222, 247, 30,
            23, 184, 79, 79, 221, 192, 225, 217, 142, 135, 164, 169, 255,
        ]);
        assert_eq!(result, Side::Tails);

        let result = coinflip([
            52, 187, 72, 255, 102, 110, 115, 233, 50, 165, 124, 255, 217, 131, 112, 209, 253, 176,
            108, 99, 102, 225, 12, 36, 82, 107, 106, 207, 99, 107, 197, 84,
        ]);
        assert_eq!(result, Side::Heads);
    }
}
