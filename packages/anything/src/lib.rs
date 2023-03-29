//! A minimal (like seriously), zero dependency protobuf encoder.
//!
//! Supported:
//! - Varint (u64)
//! - Repeated: Just append a field multiple times
//! - Nested: Just append an `Anything` instance
//!
//! Non supported:
//!
//! - Fixed length types
//! - Field sorting

#[derive(Default)]
pub struct Anything {
    output: Vec<u8>,
}

/// The protobuf wire types
///
/// <https://protobuf.dev/programming-guides/encoding/#structure>
#[repr(u32)]
enum WireType {
    /// Variable length field (int32, int64, uint32, uint64, sint32, sint64, bool, enum)
    Varint = 0,
    // Fixed length types unsupported
    // I64 = 1,
    /// Lengths prefixed field (string, bytes, embedded messages, packed repeated fields)
    Len = 2,
    // group start/end (deprecated, unsupported)
    // SGROUP = 3,
    // EGROUP = 4,
    // Fixed length types unsupported
    // I32 = 5,
}

fn varint_encode(mut n: u64, dest: &mut Vec<u8>) {
    let mut buf = [0u8; 10];
    let mut len = 0;
    loop {
        // Read least significant 7 bits
        let mut b = (n & 0b0111_1111) as u8;
        n >>= 7;
        // Set top bit when not yet done
        if n != 0 {
            b |= 0b1000_0000;
        }
        buf[len] = b;
        len += 1;
        if n == 0 {
            break;
        }
    }
    dest.extend_from_slice(&buf[0..len]);
}

impl Anything {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append_bytes(mut self, field_number: u32, data: impl AsRef<[u8]>) -> Self {
        let data = data.as_ref();
        if data.is_empty() {
            return self;
        }
        // tag
        self.append_tag(field_number, WireType::Len);
        // length
        varint_encode(data.len() as u64, &mut self.output);
        // value
        self.output.extend_from_slice(data);
        self
    }

    /// Appends a uint64 field to with the given field number.
    pub fn append_uint64(mut self, field_number: u32, value: u64) -> Self {
        if value == 0 {
            return self;
        }
        self.append_tag(field_number, WireType::Varint);
        varint_encode(value, &mut self.output);
        self
    }

    /// Appends a uint32 field to with the given field number.
    #[inline]
    pub fn append_uint32(self, field_number: u32, value: u32) -> Self {
        self.append_uint64(field_number, value.into())
    }

    /// Appends a bool field to with the given field number.
    #[inline]
    pub fn append_bool(self, field_number: u32, value: bool) -> Self {
        self.append_uint64(field_number, value.into())
    }

    /// Appends a nested protobuf message with the given field number.
    pub fn append_message(self, field_number: u32, value: &Anything) -> Self {
        self.append_bytes(field_number, value.as_bytes())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.output
    }

    /// Takes the instance and returns the protobuf bytes
    pub fn into_vec(self) -> Vec<u8> {
        self.output
    }

    fn append_tag(&mut self, field_number: u32, field_type: WireType) {
        // The top 3 bits of a field number must be unset, ie.e this shift is safe for valid field numbers
        // "The smallest field number you can specify is 1, and the largest is 2^29-1, or 536,870,911"
        // https://protobuf.dev/programming-guides/proto3/#assigning-field-numbers
        let tag: u32 = (field_number << 3) | field_type as u32;
        varint_encode(tag as u64, &mut self.output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_returns_empty_data() {
        let data = Anything::new();
        assert_eq!(data.into_vec(), &[]);
    }

    #[test]
    fn append_uint64_works() {
        let data = Anything::new().append_uint64(1, 150);
        assert_eq!(data.into_vec(), [0b00001000, 0b10010110, 0b00000001]);

        // Zero/Default field not written
        let data = Anything::new().append_uint64(1, 0);
        assert_eq!(data.into_vec(), &[]);
    }

    #[test]
    fn append_uint32_works() {
        let data = Anything::new().append_uint32(1, 150);
        assert_eq!(data.into_vec(), [0b00001000, 0b10010110, 0b00000001]);

        // large value (echo "number: 215874321" | protoc --encode=Room *.proto | hexdump -C)
        let data = Anything::new().append_uint32(1, 215874321);
        assert_eq!(data.into_vec(), b"\x08\x91\xf6\xf7\x66");

        // max value (echo "number: 4294967295" | protoc --encode=Room *.proto | hexdump -C)
        let data = Anything::new().append_uint32(1, u32::MAX);
        assert_eq!(data.into_vec(), b"\x08\xff\xff\xff\xff\x0f");

        // Zero/Default field not written
        let data = Anything::new().append_uint32(1, 0);
        assert_eq!(data.into_vec(), &[]);
    }

    #[test]
    fn append_bool_works() {
        // echo "on: true" | protoc --encode=Lights *.proto | hexdump -C
        let data = Anything::new().append_bool(3, true);
        assert_eq!(data.into_vec(), [0x18, 0x01]);

        // Zero/Default field not written
        let data = Anything::new().append_bool(3, false);
        assert_eq!(data.into_vec(), &[]);
    }

    #[test]
    fn append_bytes() {
        // &str
        let data = Anything::new().append_bytes(2, "testing");
        assert_eq!(
            data.into_vec(),
            [0x12, 0x07, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6e, 0x67]
        );

        // String
        let data = Anything::new().append_bytes(2, String::from("testing"));
        assert_eq!(
            data.into_vec(),
            [0x12, 0x07, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6e, 0x67]
        );

        // &[u8]
        let data = Anything::new().append_bytes(2, b"testing");
        assert_eq!(
            data.into_vec(),
            [0x12, 0x07, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6e, 0x67]
        );

        // Empty field not written
        let data = Anything::new().append_bytes(2, b"");
        assert_eq!(data.into_vec(), []);
    }

    #[test]
    fn append_message_works() {
        // echo "number: 4; lights: {on: true}; size: 56" | protoc --encode=Room *.proto | hexdump -C
        let data = Anything::new()
            .append_uint64(1, 4)
            .append_message(2, &Anything::new().append_bool(3, true))
            .append_uint64(3, 56);
        assert_eq!(data.into_vec(), b"\x08\x04\x12\x02\x18\x01\x18\x38");
    }
}
