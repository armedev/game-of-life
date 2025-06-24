use anyhow::{Result, bail};
use axum_tws::{Message, Payload};
use tracing::debug;

pub const PROTOCOL_VERSION: u8 = 1;
pub const HEADER_LENGTH: u8 = 7;

#[derive(Debug)]
pub struct WsMessage {
    pub version: u8,
    pub msg_type: u8,
    pub flags: u8,
    pub payload: Vec<u8>,
}

pub fn decode_ws_message(data: Payload) -> Result<WsMessage> {
    let data_len = data.len();
    debug!("Decoding WebSocket message of {} bytes", data_len);

    if data_len < HEADER_LENGTH as usize {
        bail!(
            "Message too short: {} bytes (minimum {} required for header)",
            data_len,
            HEADER_LENGTH
        );
    }

    let version = data[0];
    if version != PROTOCOL_VERSION {
        bail!(
            "Unsupported protocol version: {} (expected {})",
            version,
            PROTOCOL_VERSION
        );
    }

    let msg_type = data[1];
    let flags = data[2];
    let payload_length = u32::from_be_bytes([data[3], data[4], data[5], data[6]]) as usize;
    let expected_total_length = HEADER_LENGTH as usize + payload_length;

    if data_len != expected_total_length {
        bail!(
            "Message length mismatch: got {} bytes, expected {} bytes (header: {}, payload: {})",
            data_len,
            expected_total_length,
            HEADER_LENGTH,
            payload_length
        );
    }

    let payload = data[HEADER_LENGTH as usize..].to_vec();

    debug!(
        "Successfully decoded message: version={}, type={}, flags={}, payload_len={}",
        version,
        msg_type,
        flags,
        payload.len()
    );

    Ok(WsMessage {
        version,
        msg_type,
        flags,
        payload,
    })
}

pub fn encode_ws_message(msg: &WsMessage) -> Message {
    let total_size = HEADER_LENGTH as usize + msg.payload.len();
    let mut buf = Vec::with_capacity(total_size);

    buf.push(msg.version);
    buf.push(msg.msg_type);
    buf.push(msg.flags);
    buf.extend(&(msg.payload.len() as u32).to_be_bytes());
    buf.extend(&msg.payload);

    debug!(
        "Encoded message: version={}, type={}, flags={}, total_size={}",
        msg.version, msg.msg_type, msg.flags, total_size
    );

    Message::binary(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    fn encode_decode_roundtrip() {
        let msg = WsMessage {
            version: 1,
            msg_type: 42,
            flags: 1,
            payload: b"hello world".to_vec(),
        };

        let buf = encode_ws_message(&msg);
        let decoded = decode_ws_message(buf.into_payload()).unwrap();

        assert_eq!(msg.version, decoded.version);
        assert_eq!(msg.msg_type, decoded.msg_type);
        assert_eq!(msg.flags, decoded.flags);
        assert_eq!(msg.payload, decoded.payload);
    }

    #[test]
    #[traced_test]
    fn decode_invalid_version() {
        let mut data = vec![2, 42, 0, 0, 0, 0, 5]; // version 2 (invalid)
        data.extend(b"hello");

        let result = decode_ws_message(data.into());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported protocol version")
        );
    }

    #[test]
    #[traced_test]
    fn decode_message_too_short() {
        let data = vec![1, 42, 0, 0, 0, 0]; // Only 6 bytes, need at least 7

        let result = decode_ws_message(data.into());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Message too short")
        );
    }

    #[test]
    #[traced_test]
    fn decode_empty_message() {
        let data = vec![];

        let result = decode_ws_message(data.into());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Message too short")
        );
    }

    #[test]
    #[traced_test]
    fn decode_minimum_valid_message() {
        let data = vec![1, 100, 255, 0, 0, 0, 0]; // No payload

        let decoded = decode_ws_message(data.into()).unwrap();
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.msg_type, 100);
        assert_eq!(decoded.flags, 255);
        assert_eq!(decoded.payload.len(), 0);
    }

    #[test]
    #[traced_test]
    fn decode_payload_length_mismatch_too_short() {
        let mut data = vec![1, 42, 0, 0, 0, 0, 10]; // Claims 10 bytes payload
        data.extend(b"hello"); // Only 5 bytes payload

        let result = decode_ws_message(data.into());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Message length mismatch")
        );
    }

    #[test]
    #[traced_test]
    fn decode_payload_length_mismatch_too_long() {
        let mut data = vec![1, 42, 0, 0, 0, 0, 3]; // Claims 3 bytes payload
        data.extend(b"hello world"); // 11 bytes payload

        let result = decode_ws_message(data.into());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Message length mismatch")
        );
    }

    #[test]
    #[traced_test]
    fn encode_decode_large_payload() {
        let large_payload = vec![0xAB; 1024]; // 1KB of 0xAB bytes
        let msg = WsMessage {
            version: 1,
            msg_type: 255,
            flags: 128,
            payload: large_payload.clone(),
        };

        let encoded = encode_ws_message(&msg);
        let decoded = decode_ws_message(encoded.into_payload()).unwrap();

        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.msg_type, 255);
        assert_eq!(decoded.flags, 128);
        assert_eq!(decoded.payload, large_payload);
    }

    #[test]
    #[traced_test]
    fn encode_decode_max_u32_payload_size() {
        // Test with a reasonably large payload (not full u32::MAX for practical reasons)
        let payload_size = 65536; // 64KB
        let payload = vec![0x42; payload_size];
        let msg = WsMessage {
            version: 1,
            msg_type: 200,
            flags: 64,
            payload: payload.clone(),
        };

        let encoded = encode_ws_message(&msg);
        let decoded = decode_ws_message(encoded.into_payload()).unwrap();

        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.msg_type, 200);
        assert_eq!(decoded.flags, 64);
        assert_eq!(decoded.payload, payload);
        assert_eq!(decoded.payload.len(), payload_size);
    }

    #[test]
    #[traced_test]
    fn encode_decode_binary_payload() {
        let binary_payload = vec![0, 1, 2, 3, 255, 254, 253, 128, 127];
        let msg = WsMessage {
            version: 1,
            msg_type: 50,
            flags: 0,
            payload: binary_payload.clone(),
        };

        let encoded = encode_ws_message(&msg);
        let decoded = decode_ws_message(encoded.into_payload()).unwrap();

        assert_eq!(decoded.payload, binary_payload);
    }

    #[test]
    #[traced_test]
    fn encode_decode_all_flag_values() {
        for flags in 0..=255u8 {
            let msg = WsMessage {
                version: 1,
                msg_type: 10,
                flags,
                payload: vec![flags], // Use flag value as payload for verification
            };

            let encoded = encode_ws_message(&msg);
            let decoded = decode_ws_message(encoded.into_payload()).unwrap();

            assert_eq!(decoded.flags, flags);
            assert_eq!(decoded.payload, vec![flags]);
        }
    }

    #[test]
    #[traced_test]
    fn encode_decode_all_message_types() {
        for msg_type in 0..=255u8 {
            let msg = WsMessage {
                version: 1,
                msg_type,
                flags: 0,
                payload: vec![msg_type], // Use msg_type as payload for verification
            };

            let encoded = encode_ws_message(&msg);
            let decoded = decode_ws_message(encoded.into_payload()).unwrap();

            assert_eq!(decoded.msg_type, msg_type);
            assert_eq!(decoded.payload, vec![msg_type]);
        }
    }

    #[test]
    #[traced_test]
    fn decode_invalid_version_zero() {
        let data = vec![0, 42, 0, 0, 0, 0, 0]; // version 0

        let result = decode_ws_message(data.into());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported protocol version: 0")
        );
    }

    #[test]
    #[traced_test]
    fn decode_invalid_version_max() {
        let data = vec![255, 42, 0, 0, 0, 0, 0]; // version 255

        let result = decode_ws_message(data.into());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported protocol version: 255")
        );
    }

    #[test]
    #[traced_test]
    fn encode_message_capacity_optimization() {
        let payload = b"test payload".to_vec();
        let msg = WsMessage {
            version: 1,
            msg_type: 1,
            flags: 0,
            payload: payload.clone(),
        };

        let encoded = encode_ws_message(&msg);
        let encoded_data = encoded.into_payload();

        // Verify the encoded data has correct structure
        assert_eq!(encoded_data[0], 1); // version
        assert_eq!(encoded_data[1], 1); // msg_type
        assert_eq!(encoded_data[2], 0); // flags

        // Verify payload length encoding (big-endian u32)
        let payload_len = u32::from_be_bytes([
            encoded_data[3],
            encoded_data[4],
            encoded_data[5],
            encoded_data[6],
        ]);
        assert_eq!(payload_len as usize, payload.len());

        // Verify payload data
        assert_eq!(&encoded_data[7..], &payload);
    }

    #[test]
    #[traced_test]
    fn decode_utf8_string_payload() {
        let utf8_string = "Hello, 世界! 🌍";
        let msg = WsMessage {
            version: 1,
            msg_type: 1,
            flags: 0,
            payload: utf8_string.as_bytes().to_vec(),
        };

        let encoded = encode_ws_message(&msg);
        let decoded = decode_ws_message(encoded.into_payload()).unwrap();

        assert_eq!(decoded.payload, utf8_string.as_bytes());
        assert_eq!(String::from_utf8(decoded.payload).unwrap(), utf8_string);
    }

    #[test]
    #[traced_test]
    fn decode_header_only_truncated() {
        // Test various truncated headers
        for len in 1..7 {
            let data = vec![1; len];
            let result = decode_ws_message(data.into());
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Message too short")
            );
        }
    }
}
