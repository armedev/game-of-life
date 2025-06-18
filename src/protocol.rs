use anyhow::{Result, bail};
use axum_tws::{Message, Payload};

pub const PROTOCOL_VERSION: u8 = 1;

#[derive(Debug)]
pub struct WsMessage {
    pub version: u8,
    pub msg_type: u8,
    pub flags: u8,
    pub payload: Vec<u8>,
}

pub fn decode_ws_message(data: Payload) -> Result<WsMessage> {
    if data.len() < 7 {
        bail!("Too short for header");
    }

    let version = data[0];
    if version != 1 {
        bail!("Unsupported version: {}", version);
    }

    let msg_type = data[1];
    let flags = data[2];
    let length = u32::from_be_bytes([data[3], data[4], data[5], data[6]]) as usize;

    if data.len() != 7 + length {
        bail!("Length mismatch");
    }

    Ok(WsMessage {
        version,
        msg_type,
        flags,
        payload: data[7..].to_vec(),
    })
}

pub fn encode_ws_message(msg: &WsMessage) -> Message {
    let mut buf = Vec::with_capacity(7 + msg.payload.len());
    buf.push(msg.version);
    buf.push(msg.msg_type);
    buf.push(msg.flags);
    buf.extend(&(msg.payload.len() as u32).to_be_bytes());
    buf.extend(&msg.payload);
    Message::binary(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let msg = WsMessage {
            version: 1,
            msg_type: 42,
            flags: 1,
            payload: b"hello world".to_vec(),
        };

        let buf = encode_ws_message(&msg);
        buf.clone()
            .into_payload()
            .to_vec()
            .iter()
            .for_each(|a| print!("{:X} ", a));
        println!("");

        let decoded = decode_ws_message(buf.into_payload()).unwrap();

        assert_eq!(msg.version, decoded.version);
        assert_eq!(msg.msg_type, decoded.msg_type);
        assert_eq!(msg.flags, decoded.flags);
        assert_eq!(msg.payload, decoded.payload);
    }
}
