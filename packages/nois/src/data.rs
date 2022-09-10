use std::fmt;
use std::ops::Deref;

use schemars::JsonSchema;
use serde::{de, ser, Deserialize, Deserializer, Serialize};

use cosmwasm_std::{StdError, StdResult};

/// Data is a wrapper around Vec<u8> to add hex de/serialization
/// with serde. It also adds some helper methods to help encode inline.
///
/// This is similar to `cosmwasm_stad::Binary` but uses hex.
#[derive(Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, JsonSchema)]
pub struct Data(#[schemars(with = "String")] pub Vec<u8>);

impl Data {
    pub fn from_hex(input: &str) -> StdResult<Self> {
        let vec =
            hex::decode(input).map_err(|e| StdError::generic_err(format!("Invalid hex: {e}")))?;
        Ok(Self(vec))
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Copies content into fixed-sized array.
    ///
    /// # Examples
    ///
    /// Copy to array of explicit length
    ///
    /// ```
    /// # use nois::Data;
    /// let data = Data::from(&[0xfb, 0x1f, 0x37]);
    /// let array: [u8; 3] = data.to_array().unwrap();
    /// assert_eq!(array, [0xfb, 0x1f, 0x37]);
    /// ```
    ///
    /// Copy to integer
    ///
    /// ```
    /// # use nois::Data;
    /// let data = Data::from(&[0x8b, 0x67, 0x64, 0x84, 0xb5, 0xfb, 0x1f, 0x37]);
    /// let num = u64::from_be_bytes(data.to_array().unwrap());
    /// assert_eq!(num, 10045108015024774967);
    /// ```
    pub fn to_array<const LENGTH: usize>(&self) -> StdResult<[u8; LENGTH]> {
        if self.len() != LENGTH {
            return Err(StdError::invalid_data_size(LENGTH, self.len()));
        }

        let mut out: [u8; LENGTH] = [0; LENGTH];
        out.copy_from_slice(&self.0);
        Ok(out)
    }
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use an output inspired by tuples (https://doc.rust-lang.org/std/fmt/struct.Formatter.html#method.debug_tuple)
        // but with a custom implementation to avoid the need for an intemediate hex string.
        write!(f, "Data(")?;
        for byte in self.0.iter() {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, ")")?;
        Ok(())
    }
}

impl From<&[u8]> for Data {
    fn from(binary: &[u8]) -> Self {
        Self(binary.to_vec())
    }
}

/// Just like Vec<u8>, Data is a smart pointer to [u8].
/// This implements `*data` for us and allows us to
/// do `&*data`, returning a `&[u8]` from a `&Data`.
/// With [deref coercions](https://doc.rust-lang.org/1.22.1/book/first-edition/deref-coercions.html#deref-coercions),
/// this allows us to use `&data` whenever a `&[u8]` is required.
impl Deref for Data {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

// Reference
impl<const LENGTH: usize> From<&[u8; LENGTH]> for Data {
    fn from(source: &[u8; LENGTH]) -> Self {
        Self(source.to_vec())
    }
}

// Owned
impl<const LENGTH: usize> From<[u8; LENGTH]> for Data {
    fn from(source: [u8; LENGTH]) -> Self {
        Self(source.into())
    }
}

impl From<Vec<u8>> for Data {
    fn from(vec: Vec<u8>) -> Self {
        Self(vec)
    }
}

impl From<Data> for Vec<u8> {
    fn from(original: Data) -> Vec<u8> {
        original.0
    }
}

/// Implement `Data == std::vec::Vec<u8>`
impl PartialEq<Vec<u8>> for Data {
    fn eq(&self, rhs: &Vec<u8>) -> bool {
        // Use Vec<u8> == Vec<u8>
        self.0 == *rhs
    }
}

/// Implement `std::vec::Vec<u8> == Data`
impl PartialEq<Data> for Vec<u8> {
    fn eq(&self, rhs: &Data) -> bool {
        // Use Vec<u8> == Vec<u8>
        *self == rhs.0
    }
}

/// Implement `Data == &[u8]`
impl PartialEq<&[u8]> for Data {
    fn eq(&self, rhs: &&[u8]) -> bool {
        // Use &[u8] == &[u8]
        self.as_slice() == *rhs
    }
}

/// Implement `&[u8] == Data`
impl PartialEq<Data> for &[u8] {
    fn eq(&self, rhs: &Data) -> bool {
        // Use &[u8] == &[u8]
        *self == rhs.as_slice()
    }
}

/// Implement `Data == [u8; LENGTH]`
impl<const LENGTH: usize> PartialEq<[u8; LENGTH]> for Data {
    fn eq(&self, rhs: &[u8; LENGTH]) -> bool {
        self.as_slice() == rhs.as_slice()
    }
}

/// Implement `[u8; LENGTH] == Data`
impl<const LENGTH: usize> PartialEq<Data> for [u8; LENGTH] {
    fn eq(&self, rhs: &Data) -> bool {
        self.as_slice() == rhs.as_slice()
    }
}

/// Implement `Data == &[u8; LENGTH]`
impl<const LENGTH: usize> PartialEq<&[u8; LENGTH]> for Data {
    fn eq(&self, rhs: &&[u8; LENGTH]) -> bool {
        self.as_slice() == rhs.as_slice()
    }
}

/// Implement `&[u8; LENGTH] == Data`
impl<const LENGTH: usize> PartialEq<Data> for &[u8; LENGTH] {
    fn eq(&self, rhs: &Data) -> bool {
        self.as_slice() == rhs.as_slice()
    }
}

/// Serializes as a hex string
impl Serialize for Data {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

/// Deserializes as a hex string
impl<'de> Deserialize<'de> for Data {
    fn deserialize<D>(deserializer: D) -> Result<Data, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(HexVisitor)
    }
}

struct HexVisitor;

impl<'de> de::Visitor<'de> for HexVisitor {
    type Value = Data;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("valid hex encoded string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match Data::from_hex(v) {
            Ok(data) => Ok(data),
            Err(_) => Err(E::custom(format!("invalid hex: {}", v))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{from_slice, to_vec, StdError};
    use std::collections::hash_map::DefaultHasher;
    use std::collections::HashSet;
    use std::hash::{Hash, Hasher};

    #[test]
    fn from_hex_works() {
        let data = Data::from_hex("").unwrap();
        assert_eq!(data, b"");
        let data = Data::from_hex("61").unwrap();
        assert_eq!(data, b"a");
        let data = Data::from_hex("00").unwrap();
        assert_eq!(data, b"\0");

        let data = Data::from_hex("68656c6c6f").unwrap();
        assert_eq!(data, b"hello");
        let data = Data::from_hex("68656C6C6F").unwrap();
        assert_eq!(data, b"hello");
        let data = Data::from_hex("72616e646f6d695a").unwrap();
        assert_eq!(data.as_slice(), b"randomiZ");

        // odd
        match Data::from_hex("123").unwrap_err() {
            StdError::GenericErr { msg, .. } => {
                assert_eq!(msg, "Invalid hex: Odd number of digits")
            }
            _ => panic!("Unexpected error type"),
        }
        // non-hex
        match Data::from_hex("efgh").unwrap_err() {
            StdError::GenericErr { msg, .. } => {
                assert_eq!(msg, "Invalid hex: Invalid character 'g' at position 2")
            }
            _ => panic!("Unexpected error type"),
        }
        // spaces
        Data::from_hex("aa ").unwrap_err();
        Data::from_hex(" aa").unwrap_err();
        Data::from_hex("a a").unwrap_err();
        Data::from_hex(" aa ").unwrap_err();
    }

    #[test]
    fn to_hex_works() {
        let binary: &[u8] = b"";
        let encoded = Data::from(binary).to_hex();
        assert_eq!(encoded, "");

        let binary: &[u8] = b"hello";
        let encoded = Data::from(binary).to_hex();
        assert_eq!(encoded, "68656c6c6f");

        let binary = vec![12u8, 187, 0, 17, 250, 1];
        let encoded = Data(binary).to_hex();
        assert_eq!(encoded, "0cbb0011fa01");
    }

    #[test]
    fn to_array_works() {
        // simple
        let binary = Data::from(&[1, 2, 3]);
        let array: [u8; 3] = binary.to_array().unwrap();
        assert_eq!(array, [1, 2, 3]);

        // empty
        let binary = Data::from(&[]);
        let array: [u8; 0] = binary.to_array().unwrap();
        assert_eq!(array, [] as [u8; 0]);

        // invalid size
        let binary = Data::from(&[1, 2, 3]);
        let error = binary.to_array::<8>().unwrap_err();
        match error {
            StdError::InvalidDataSize {
                expected, actual, ..
            } => {
                assert_eq!(expected, 8);
                assert_eq!(actual, 3);
            }
            err => panic!("Unexpected error: {:?}", err),
        }

        // long array (32 bytes)
        let binary =
            Data::from_hex("b75d7d24e428c7859440498efe7caa3997cefb08c99bdd581b6b1f9f866096f0")
                .unwrap();
        let array: [u8; 32] = binary.to_array().unwrap();
        assert_eq!(
            array,
            [
                0xb7, 0x5d, 0x7d, 0x24, 0xe4, 0x28, 0xc7, 0x85, 0x94, 0x40, 0x49, 0x8e, 0xfe, 0x7c,
                0xaa, 0x39, 0x97, 0xce, 0xfb, 0x08, 0xc9, 0x9b, 0xdd, 0x58, 0x1b, 0x6b, 0x1f, 0x9f,
                0x86, 0x60, 0x96, 0xf0,
            ]
        );

        // very long array > 32 bytes (requires Rust 1.47+)
        let binary = Data::from_hex(
            "b75d7d24e428c7859440498efe7caa3997cefb08c99bdd581b6b1f9f866096f073c8c3b0316abe",
        )
        .unwrap();
        let array: [u8; 39] = binary.to_array().unwrap();
        assert_eq!(
            array,
            [
                0xb7, 0x5d, 0x7d, 0x24, 0xe4, 0x28, 0xc7, 0x85, 0x94, 0x40, 0x49, 0x8e, 0xfe, 0x7c,
                0xaa, 0x39, 0x97, 0xce, 0xfb, 0x08, 0xc9, 0x9b, 0xdd, 0x58, 0x1b, 0x6b, 0x1f, 0x9f,
                0x86, 0x60, 0x96, 0xf0, 0x73, 0xc8, 0xc3, 0xb0, 0x31, 0x6a, 0xbe,
            ]
        );
    }

    #[test]
    fn from_slice_works() {
        let original: &[u8] = &[0u8, 187, 61, 11, 250, 0];
        let binary: Data = original.into();
        assert_eq!(binary.as_slice(), [0u8, 187, 61, 11, 250, 0]);
    }

    #[test]
    fn from_fixed_length_array_works() {
        let original = &[];
        let binary: Data = original.into();
        assert_eq!(binary.len(), 0);

        let original = &[0u8];
        let binary: Data = original.into();
        assert_eq!(binary.as_slice(), [0u8]);

        let original = &[0u8, 187, 61, 11, 250, 0];
        let binary: Data = original.into();
        assert_eq!(binary.as_slice(), [0u8, 187, 61, 11, 250, 0]);

        let original = &[
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1,
        ];
        let binary: Data = original.into();
        assert_eq!(
            binary.as_slice(),
            [
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1,
            ]
        );
    }

    #[test]
    fn from_owned_fixed_length_array_works() {
        let original = [];
        let binary: Data = original.into();
        assert_eq!(binary.len(), 0);

        let original = [0u8];
        let binary: Data = original.into();
        assert_eq!(binary.as_slice(), [0u8]);

        let original = [0u8, 187, 61, 11, 250, 0];
        let binary: Data = original.into();
        assert_eq!(binary.as_slice(), [0u8, 187, 61, 11, 250, 0]);

        let original = [
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1,
        ];
        let binary: Data = original.into();
        assert_eq!(
            binary.as_slice(),
            [
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1,
            ]
        );
    }

    #[test]
    fn from_literal_works() {
        let a: Data = b"".into();
        assert_eq!(a.len(), 0);

        let a: Data = b".".into();
        assert_eq!(a.len(), 1);

        let a: Data = b"...".into();
        assert_eq!(a.len(), 3);

        let a: Data = b"...............................".into();
        assert_eq!(a.len(), 31);

        let a: Data = b"................................".into();
        assert_eq!(a.len(), 32);

        let a: Data = (b".................................").into();
        assert_eq!(a.len(), 33);
    }

    #[test]
    fn from_vec_works() {
        let original = vec![0u8, 187, 61, 11, 250, 0];
        let original_ptr = original.as_ptr();
        let binary: Data = original.into();
        assert_eq!(binary.as_slice(), [0u8, 187, 61, 11, 250, 0]);
        assert_eq!(binary.0.as_ptr(), original_ptr, "vector must not be copied");
    }

    #[test]
    fn into_vec_works() {
        // Into<Vec<u8>> for Data
        let original = Data(vec![0u8, 187, 61, 11, 250, 0]);
        let original_ptr = original.0.as_ptr();
        let vec: Vec<u8> = original.into();
        assert_eq!(vec.as_slice(), [0u8, 187, 61, 11, 250, 0]);
        assert_eq!(vec.as_ptr(), original_ptr, "vector must not be copied");

        // From<Data> for Vec<u8>
        let original = Data(vec![7u8, 35, 49, 101, 0, 255]);
        let original_ptr = original.0.as_ptr();
        let vec = Vec::<u8>::from(original);
        assert_eq!(vec.as_slice(), [7u8, 35, 49, 101, 0, 255]);
        assert_eq!(vec.as_ptr(), original_ptr, "vector must not be copied");
    }

    #[test]
    fn serialization_works() {
        let binary = Data(vec![0u8, 187, 61, 11, 250, 0]);

        let json = to_vec(&binary).unwrap();
        let deserialized: Data = from_slice(&json).unwrap();

        assert_eq!(binary, deserialized);
    }

    #[test]
    fn deserialize_from_valid_string() {
        let hex = "00bb3d0bfa00";
        // this is the binary behind above string
        let expected = vec![0u8, 187, 61, 11, 250, 0];

        let serialized = to_vec(&hex).unwrap();
        let deserialized: Data = from_slice(&serialized).unwrap();
        assert_eq!(expected, deserialized.as_slice());
    }

    #[test]
    fn deserialize_from_invalid_string() {
        let invalid_str = "**BAD!**";
        let serialized = to_vec(&invalid_str).unwrap();
        let res = from_slice::<Data>(&serialized);
        assert!(res.is_err());
    }

    #[test]
    fn data_implements_debug() {
        // Some data
        let data = Data(vec![0x07, 0x35, 0xAA, 0xcb, 0x00, 0xff]);
        assert_eq!(format!("{:?}", data), "Data(0735aacb00ff)",);

        // Empty
        let data = Data(vec![]);
        assert_eq!(format!("{:?}", data), "Data()",);
    }

    #[test]
    fn data_implements_deref() {
        // Dereference to [u8]
        let data = Data(vec![7u8, 35, 49, 101, 0, 255]);
        assert_eq!(*data, [7u8, 35, 49, 101, 0, 255]);

        // This checks deref coercions from &Binary to &[u8] works
        let data = Data(vec![7u8, 35, 49, 101, 0, 255]);
        assert_eq!(data.len(), 6);
        let data_slice: &[u8] = &data;
        assert_eq!(data_slice, &[7u8, 35, 49, 101, 0, 255]);
    }

    #[test]
    fn data_implements_hash() {
        let a1 = Data::from([0, 187, 61, 11, 250, 0]);
        let mut hasher = DefaultHasher::new();
        a1.hash(&mut hasher);
        let a1_hash = hasher.finish();

        let a2 = Data::from([0, 187, 61, 11, 250, 0]);
        let mut hasher = DefaultHasher::new();
        a2.hash(&mut hasher);
        let a2_hash = hasher.finish();

        let b = Data::from([16, 21, 33, 0, 255, 9]);
        let mut hasher = DefaultHasher::new();
        b.hash(&mut hasher);
        let b_hash = hasher.finish();

        assert_eq!(a1_hash, a2_hash);
        assert_ne!(a1_hash, b_hash);
    }

    /// This requires Hash and Eq to be implemented
    #[test]
    fn data_can_be_used_in_hash_set() {
        let a1 = Data::from([0, 187, 61, 11, 250, 0]);
        let a2 = Data::from([0, 187, 61, 11, 250, 0]);
        let b = Data::from([16, 21, 33, 0, 255, 9]);

        let mut set = HashSet::new();
        set.insert(a1.clone());
        set.insert(a2.clone());
        set.insert(b.clone());
        assert_eq!(set.len(), 2);

        let set1 = HashSet::<Data>::from_iter(vec![b.clone(), a1.clone()]);
        let set2 = HashSet::from_iter(vec![a1, a2, b]);
        assert_eq!(set1, set2);
    }

    #[test]
    fn data_implements_partial_eq_with_vector() {
        let a = Data(vec![5u8; 3]);
        let b = vec![5u8; 3];
        let c = vec![9u8; 3];
        assert_eq!(a, b);
        assert_eq!(b, a);
        assert_ne!(a, c);
        assert_ne!(c, a);
    }

    #[test]
    fn data_implements_partial_eq_with_slice_and_array() {
        let a = Data(vec![0xAA, 0xBB]);

        // Slice: &[u8]
        assert_eq!(a, b"\xAA\xBB" as &[u8]);
        assert_eq!(b"\xAA\xBB" as &[u8], a);
        assert_ne!(a, b"\x11\x22" as &[u8]);
        assert_ne!(b"\x11\x22" as &[u8], a);

        // Array reference: &[u8; 2]
        assert_eq!(a, b"\xAA\xBB");
        assert_eq!(b"\xAA\xBB", a);
        assert_ne!(a, b"\x11\x22");
        assert_ne!(b"\x11\x22", a);

        // Array: [u8; 2]
        assert_eq!(a, [0xAA, 0xBB]);
        assert_eq!([0xAA, 0xBB], a);
        assert_ne!(a, [0x11, 0x22]);
        assert_ne!([0x11, 0x22], a);
    }
}
