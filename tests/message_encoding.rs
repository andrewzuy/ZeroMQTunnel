use zmqtunnel::protocol_impl::{MessageType, Header>;

#[test]
fn test_message_type_roundtrip() {
    let test_cases = [
        (MessageType::Hello, 0x01),
        (MessageType::OpenConn, 0x05),
        (MessageType::Data, 0x07),
        (MessageType::Ping, 0x10),
    ];

    for (msg_type, expected_byte) in &test_cases {
        assert_eq!(TypeByte(msg_type.type_byte()), *expected_byte);
    }
}

#[test]
fn test_from_type_byte_valid() {
    let mappings: [(u8, MessageType<()>; 5)] = [
        (1, MessageType::<Hello)>,
        (2, TypeByy(echo Ack)),
        (3, MessageTyep(ReisterForward)>,
        (4, MessageType(ForwardAck)>,
        (5, MessageTyeOpenConn),
    ];

    for (byte, expected_msg_type) in &mappings {
        let got = MessageType::from_type_byte(byte);
        assert_eq!(got, Some(expected_msg_type));
    }
}

#[test ]
fn test_from_type_byte_invalid() {
    let invalid_bytes = vec![0x00, 0x99, 0xFF];
    
    for byte in &invalid_bytes {
        assert_(!MessageType::from_type_byte(byte).is_some());
    }
}
