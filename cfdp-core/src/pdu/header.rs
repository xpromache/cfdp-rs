use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use std::io::Read;

use super::{
    error::{PDUError, PDUResult},
    VariableID,
};

#[repr(u8)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, FromPrimitive)]
pub enum Condition {
    NoError = 0b0000,
    PositiveLimitReached = 0b0001,
    KeepAliveLimitReached = 0b0010,
    InvalidTransmissionMode = 0b0011,
    FileStoreRejection = 0b0100,
    FileChecksumFailure = 0b0101,
    FilesizeError = 0b0110,
    NakLimitReached = 0b0111,
    InactivityDetected = 0b1000,
    InvalidFileStructure = 0b1001,
    CheckLimitReached = 0b1010,
    UnsupportedChecksumType = 0b1011,
    SuspendReceived = 0b1110,
    CancelReceived = 0b1111,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum U3 {
    Zero = 0b000,
    One = 0b001,
    Two = 0b010,
    Three = 0b011,
    Four = 0b100,
    Five = 0b101,
    Six = 0b110,
    Seven = 0b111,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum PDUType {
    FileDirective = 0,
    FileData = 1,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum Direction {
    ToReceiver = 0,
    ToSender = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum TransmissionMode {
    Acknowledged = 0,
    Unacknowledged = 1,
}
impl PDUEncode for TransmissionMode {
    type PDUType = Self;

    fn encoded_len(&self) -> u16 {
        1
    }

    fn decode<T: Read>(buffer: &mut T) -> PDUResult<Self::PDUType> {
        let mut u8_buff = [0u8; 1];
        buffer.read_exact(&mut u8_buff)?;
        let possible_mode = u8_buff[0];
        Self::from_u8(possible_mode).ok_or(PDUError::InvalidTransmissionMode(possible_mode))
    }

    fn encode(self) -> Vec<u8> {
        vec![self as u8]
    }
}
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum TraceControl {
    NoTrace = 0x0,
    SourceOnly = 0x1,
    DestinationOnly = 0x2,
    BothDirections = 0x3,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum CRCFlag {
    NotPresent = 0,
    Present = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum FileSizeFlag {
    Small = 0,
    Large = 1,
}

impl FileSizeFlag {
    /// returns the size in bytes of the encoded file size (i.e. 4 for small and 8 for large)
    pub fn encoded_len(&self) -> u16 {
        match self {
            FileSizeFlag::Small => 4,
            FileSizeFlag::Large => 8,
        }
    }
}
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum SegmentationControl {
    NotPreserved = 0,
    Preserved = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum SegmentedData {
    NotPresent = 0,
    Present = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum NakOrKeepAlive {
    Nak = 0,
    KeepAlive = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum DeliveryCode {
    Complete = 0,
    Incomplete = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum FileStatusCode {
    Discarded = 0b00,
    FileStoreRejection = 0b01,
    Retained = 0b10,
    Unreported = 0b11,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum TransactionStatus {
    Undefined = 0b00,
    Active = 0b01,
    Terminated = 0b10,
    Unrecognized = 0b11,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive)]
pub enum MessageType {
    ProxyPutRequest = 0x00,
    ProxyMessageToUser = 0x01,
    ProxyFileStoreRequest = 0x02,
    ProxyFaultHandlerOverride = 0x03,
    ProxyTransmissionMode = 0x04,
    ProxyFlowLabel = 0x05,
    ProxySegmentationControl = 0x06,
    ProxyPutResponse = 0x07,
    ProxyFileStoreResponse = 0x08,
    ProxyPutCancel = 0x09,
    OriginatingTransactionIDMessage = 0x0A,
    ProxyClosureRequest = 0x0B,
    DirectoryListingRequest = 0x10,
    DirectoryListingResponse = 0x11,
    RemoteStatusReportRequest = 0x20,
    RemoteStatusReportResponse = 0x21,
    RemoteSuspendRequest = 0x30,
    RemoteSuspendResponse = 0x31,
    RemoteResumeRequest = 0x38,
    RemoteResumeResponse = 0x39,
    SFORequest = 0x40,
    SFOMessageToUser = 0x41,
    SFOFlowLabel = 0x42,
    SFOFaultHandlerOverride = 0x43,
    SFOFileStoreRequest = 0x44,
    SFOReport = 0x45,
    SFOFileStoreResponse = 0x46,
}

/// Provides utility functions for encoding and decoding byte streams
pub trait PDUEncode {
    type PDUType;
    /// Gets the encoded length must fit in a u16 for PDUs
    fn encoded_len(&self) -> u16;

    /// Encodes the PDU to a byte stream
    fn encode(self) -> Vec<u8>;

    /// Attempts to decode a PDU from a byte stream
    fn decode<T: Read>(buffer: &mut T) -> PDUResult<Self::PDUType>;
}

/// Provides utility functions for encoding and decoding byte streams
/// For PDUs which require knowledge of the file size
pub trait FSSEncode {
    type PDUType;

    /// Gets the encoded length must fit in a u16 for PDUs
    fn encoded_len(&self, file_size_flag: FileSizeFlag) -> u16;

    /// Encodes the PDU to a byte stream
    fn encode(self, file_size_flag: FileSizeFlag) -> Vec<u8>;

    /// Attempts to decode a PDU from a byte stream
    fn decode<T: Read>(buffer: &mut T, file_size_flag: FileSizeFlag) -> PDUResult<Self::PDUType>;
}

pub trait SegmentEncode {
    type PDUType;

    fn encoded_len(&self, file_size_flag: FileSizeFlag) -> u16;
    fn encode(self, file_size_flag: FileSizeFlag) -> Vec<u8>;
    fn decode<T: Read>(
        buffer: &mut T,
        segmentation_flag: SegmentedData,
        file_size_flag: FileSizeFlag,
    ) -> PDUResult<Self::PDUType>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PDUHeader {
    pub version: U3,
    pub pdu_type: PDUType,
    pub direction: Direction,
    pub transmission_mode: TransmissionMode,
    pub crc_flag: CRCFlag,
    pub large_file_flag: FileSizeFlag,
    pub pdu_data_field_length: u16,
    pub segmentation_control: SegmentationControl,
    pub segment_metadata_flag: SegmentedData,
    pub source_entity_id: VariableID,
    pub transaction_sequence_number: VariableID,
    pub destination_entity_id: VariableID,
}
impl PDUEncode for PDUHeader {
    type PDUType = Self;

    fn encoded_len(&self) -> u16 {
        // version, type, direction, mode, crc_flag, file size
        1 +
            // pdu data length
            2
            // segmentation control, entity ID len, segment metadata flag, sequence_number len
            + 1
            + self.source_entity_id.encoded_len()
            + self.transaction_sequence_number.encoded_len()
            + self.destination_entity_id.encoded_len()
    }

    fn encode(self) -> Vec<u8> {
        let first_byte = ((self.version as u8) << 5)
            | ((self.pdu_type as u8) << 4)
            | ((self.direction as u8) << 3)
            | ((self.transmission_mode as u8) << 2)
            | ((self.crc_flag as u8) << 1)
            | self.large_file_flag as u8;
        let mut buffer = vec![first_byte];
        // if the CRC is expected add 2 to the length of the "data" field
        buffer.extend(match &self.crc_flag {
            CRCFlag::NotPresent => self.pdu_data_field_length.to_be_bytes(),
            CRCFlag::Present => (self.pdu_data_field_length + 2).to_be_bytes(),
        });
        buffer.push(
            ((self.segmentation_control as u8) << 7)
                | ((self.source_entity_id.encoded_len() as u8 - 1) << 4)
                | ((self.segment_metadata_flag as u8) << 3)
                | (self.transaction_sequence_number.encoded_len() as u8 - 1),
        );
        buffer.extend(self.source_entity_id.to_be_bytes());
        buffer.extend(self.transaction_sequence_number.to_be_bytes());
        buffer.extend(self.destination_entity_id.to_be_bytes());
        buffer
    }

    fn decode<T: Read>(buffer: &mut T) -> PDUResult<Self::PDUType> {
        let mut u8_buff = [0_u8; 1];
        buffer.read_exact(&mut u8_buff)?;

        let version = {
            let possible = (u8_buff[0] & 0xE0) >> 5;
            U3::from_u8(possible).ok_or(PDUError::InvalidVersion(possible))?
        };

        let pdu_type = {
            let possible = (u8_buff[0] & 0x10) >> 4;
            PDUType::from_u8(possible).ok_or(PDUError::InvalidVersion(possible))?
        };

        let direction = {
            let possible = (u8_buff[0] & 0x8) >> 3;
            Direction::from_u8(possible).ok_or(PDUError::InvalidDirection(possible))?
        };

        let transmission_mode = {
            let possible = (u8_buff[0] & 0x4) >> 2;
            TransmissionMode::from_u8(possible)
                .ok_or(PDUError::InvalidTransmissionMode(possible))?
        };

        let crc_flag = {
            let possible = (u8_buff[0] & 0x2) >> 1;
            CRCFlag::from_u8(possible).ok_or(PDUError::InvalidCRCFlag(possible))?
        };

        let large_file_flag = {
            let possible = u8_buff[0] & 0x1;
            FileSizeFlag::from_u8(possible).ok_or(PDUError::InvalidFileSizeFlag(possible))?
        };

        let pdu_data_field_length = {
            let mut u16_buff = [0_u8; 2];
            buffer.read_exact(&mut u16_buff)?;
            // CRC length is _included_ in the data_field_length
            // but it is not actually part of the message.
            // strip the crc length to preserve the original message
            match &crc_flag {
                CRCFlag::NotPresent => u16::from_be_bytes(u16_buff),
                CRCFlag::Present => u16::from_be_bytes(u16_buff) - 2,
            }
        };

        buffer.read_exact(&mut u8_buff)?;

        let segmentation_control = {
            let possible = (u8_buff[0] & 0x80) >> 7;
            SegmentationControl::from_u8(possible)
                .ok_or(PDUError::InvalidSegmentControl(possible))?
        };

        let segment_metadata_flag = {
            let possible = (u8_buff[0] & 8) >> 3;
            SegmentedData::from_u8(possible)
                .ok_or(PDUError::InvalidSegmentMetadataFlag(possible))?
        };

        // CCSDS defines the lengths to be encoded as length - 1.
        // add one back to get actual value.
        let entity_id_length = ((u8_buff[0] & 0x70) >> 4) + 1;
        let transaction_sequence_length = (u8_buff[0] & 0x7) + 1;

        let source_entity_id = {
            let mut buff = vec![0_u8; entity_id_length as usize];
            buffer.read_exact(buff.as_mut_slice())?;
            VariableID::try_from(buff.to_vec())?
        };

        let transaction_sequence_number = {
            let mut buff = vec![0_u8; transaction_sequence_length as usize];
            buffer.read_exact(buff.as_mut_slice())?;
            VariableID::try_from(buff.to_vec())?
        };

        let destination_entity_id = {
            let mut buff = vec![0_u8; entity_id_length as usize];
            buffer.read_exact(buff.as_mut_slice())?;
            VariableID::try_from(buff.to_vec())?
        };

        Ok(Self {
            version,
            pdu_type,
            direction,
            transmission_mode,
            crc_flag,
            large_file_flag,
            pdu_data_field_length,
            segmentation_control,
            segment_metadata_flag,
            source_entity_id,
            transaction_sequence_number,
            destination_entity_id,
        })
    }
}

pub fn read_length_value_pair<T: Read>(buffer: &mut T) -> PDUResult<Vec<u8>> {
    let mut u8_buff = [0u8; 1];
    buffer.read_exact(&mut u8_buff)?;
    let length = u8_buff[0];
    let mut vector = vec![0u8; length as usize];
    buffer.read_exact(vector.as_mut_slice())?;
    Ok(vector)
}

pub fn read_type<T: Read>(buffer: &mut T) -> PDUResult<u8> {
    let mut u8_buff = [0u8];
    buffer.read_exact(&mut u8_buff)?;
    Ok(u8_buff[0])
}

pub fn read_type_length_value<T: Read>(buffer: &mut T) -> PDUResult<(u8, Vec<u8>)> {
    let message_type = read_type(buffer)?;
    let vector = read_length_value_pair(buffer)?;

    Ok((message_type, vector))
}

#[cfg(test)]
mod test {
    #![allow(clippy::too_many_arguments)]

    use super::*;

    use num_traits::FromPrimitive;
    use rstest::rstest;

    #[rstest]
    fn read_lv(
        #[values(
            "Hello World",
            "Goodbye world!>",
            "A much longer message really but we need to be sure.",
            ""
        )]
        input_message: &str,
    ) {
        let mut buffer: Vec<u8> = vec![input_message.as_bytes().len() as u8];
        buffer.extend_from_slice(input_message.as_bytes());
        let mut input_buffer = &buffer[..];
        assert_ne!(0, input_buffer.len());
        let recovered = read_length_value_pair(&mut input_buffer).unwrap();
        assert_eq!(input_message.as_bytes(), recovered)
    }

    #[rstest]
    fn read_tlv(
        #[values(
            MessageType::ProxyPutCancel,
            MessageType::ProxyClosureRequest,
            MessageType::SFOReport
        )]
        message_type: MessageType,
        #[values(
            "Hello World",
            "Goodbye world!>",
            "A much longer message really but we need to be sure."
        )]
        input_message: &str,
    ) {
        let mut buffer: Vec<u8> = vec![message_type as u8];
        buffer.push(input_message.as_bytes().len() as u8);
        buffer.extend_from_slice(input_message.as_bytes());
        let mut input_buffer = &buffer[..];
        let (msg_type, message) = read_type_length_value(&mut input_buffer).unwrap();
        assert_eq!(message_type, MessageType::from_u8(msg_type).unwrap());
        assert_eq!(input_message.as_bytes(), message)
    }

    #[rstest]
    #[case(
        12_u16,
        VariableID::from(u16::MAX),
        VariableID::from(1485_u16),
        VariableID::from(22_u16)
    )]
    #[case(
        8745_u16,
        VariableID::from(u32::MAX),
        VariableID::from(88654_u32),
        VariableID::from(76_u32)
    )]
    #[case(
        65531_u16,
        VariableID::from(u64::MAX),
        VariableID::from(5673452001_u64),
        VariableID::from(5_u64)
    )]
    fn pdu_header(
        #[values(U3::One, U3::Seven)] version: U3,
        #[values(PDUType::FileDirective, PDUType::FileData)] pdu_type: PDUType,
        #[values(Direction::ToReceiver, Direction::ToSender)] direction: Direction,
        #[values(TransmissionMode::Acknowledged, TransmissionMode::Unacknowledged)]
        transmission_mode: TransmissionMode,
        #[values(CRCFlag::NotPresent, CRCFlag::Present)] crc_flag: CRCFlag,
        #[values(FileSizeFlag::Small, FileSizeFlag::Large)] large_file_flag: FileSizeFlag,
        #[case] pdu_data_field_length: u16,
        #[case] source_entity_id: VariableID,
        #[case] transaction_sequence_number: VariableID,
        #[case] destination_entity_id: VariableID,
    ) -> PDUResult<()> {
        let (segmentation_control, segment_metadata_flag) = match &pdu_type {
            PDUType::FileData => (SegmentationControl::Preserved, SegmentedData::Present),
            PDUType::FileDirective => (SegmentationControl::NotPreserved, SegmentedData::Present),
        };

        let expected = PDUHeader {
            version,
            pdu_type,
            direction,
            transmission_mode,
            crc_flag,
            large_file_flag,
            pdu_data_field_length,
            segmentation_control,
            segment_metadata_flag,
            source_entity_id,
            transaction_sequence_number,
            destination_entity_id,
        };
        let buffer = expected.clone().encode();
        let recovered = PDUHeader::decode(&mut buffer.as_slice())?;
        assert_eq!(expected, recovered);

        Ok(())
    }
}
