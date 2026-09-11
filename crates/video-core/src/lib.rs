//! Encoded video contracts and bounded RTP/H.264 parsing.
use h264_reader::nal::{Nal, RefNal, sps::SeqParameterSet};
pub const MAX_ACCESS_UNIT: usize = 8 * 1024 * 1024;
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TransportError {
    #[error("Malformed RTP header or payload")]
    Malformed,
    #[error("Video access unit exceeds the configured safety limit")]
    Oversized,
    #[error("A video fragment was lost or arrived out of order")]
    PacketLoss,
    #[error("Unsupported H.264 aggregation/fragment type")]
    Unsupported,
    #[error("Invalid H.264 sequence parameter set")]
    InvalidSps,
}
#[derive(Debug)]
pub struct Rtp<'a> {
    pub marker: bool,
    pub payload_type: u8,
    pub sequence: u16,
    pub timestamp: u32,
    pub ssrc: u32,
    pub payload: &'a [u8],
}
impl<'a> Rtp<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, TransportError> {
        if bytes.len() < 12 || bytes[0] >> 6 != 2 {
            return Err(TransportError::Malformed);
        }
        let mut offset = 12 + (bytes[0] & 15) as usize * 4;
        if offset > bytes.len() {
            return Err(TransportError::Malformed);
        }
        if bytes[0] & 0x10 != 0 {
            let extension = bytes
                .get(offset..offset + 4)
                .ok_or(TransportError::Malformed)?;
            let words = u16::from_be_bytes([extension[2], extension[3]]) as usize;
            offset += 4 + words * 4;
        }
        let mut end = bytes.len();
        if bytes[0] & 0x20 != 0 {
            let padding = *bytes.last().ok_or(TransportError::Malformed)? as usize;
            if padding == 0 || padding > end.saturating_sub(offset) {
                return Err(TransportError::Malformed);
            }
            end -= padding;
        }
        let payload = bytes
            .get(offset..end)
            .filter(|p| !p.is_empty())
            .ok_or(TransportError::Malformed)?;
        Ok(Self {
            marker: bytes[1] & 0x80 != 0,
            payload_type: bytes[1] & 0x7f,
            sequence: u16::from_be_bytes([bytes[2], bytes[3]]),
            timestamp: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            ssrc: u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            payload,
        })
    }
}
pub struct EncodedFrame<'a> {
    pub avcc: &'a [u8],
    pub sps: &'a [u8],
    pub pps: &'a [u8],
    pub timestamp_90khz: u32,
    pub keyframe: bool,
    pub discontinuity: bool,
}
pub trait EncodedVideoSink {
    type Error: std::error::Error + Send + Sync + 'static;
    fn submit(&mut self, frame: &EncodedFrame<'_>) -> Result<(), Self::Error>;
}
pub fn h264_dimensions(sps: &[u8]) -> Result<(u32, u32), TransportError> {
    if sps.len() < 4 || sps.len() > 65535 || sps[0] & 31 != 7 {
        return Err(TransportError::InvalidSps);
    }
    let nal = RefNal::new(sps, &[], true);
    let sps =
        SeqParameterSet::from_bits(nal.rbsp_bits()).map_err(|_| TransportError::InvalidSps)?;
    let (width, height) = sps
        .pixel_dimensions()
        .map_err(|_| TransportError::InvalidSps)?;
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return Err(TransportError::InvalidSps);
    }
    Ok((width, height))
}
/// Retains one reusable compressed-frame buffer. A completed frame borrows it
/// until the next push, so consumers cannot accumulate an unbounded frame queue.
pub struct H264Assembler {
    buffer: Vec<u8>,
    sps: Vec<u8>,
    pps: Vec<u8>,
    sequence: Option<u16>,
    timestamp: Option<u32>,
    ssrc: Option<u32>,
    fragment: Option<usize>,
    damaged_timestamp: Option<u32>,
    emitted: std::collections::VecDeque<u32>,
    keyframe: bool,
    vcl: bool,
    ready: bool,
    needs_keyframe: bool,
    discontinuity: bool,
    pub dropped: u64,
}
impl Default for H264Assembler {
    fn default() -> Self {
        Self {
            buffer: Vec::with_capacity(256 * 1024),
            sps: Vec::new(),
            pps: Vec::new(),
            sequence: None,
            timestamp: None,
            ssrc: None,
            fragment: None,
            damaged_timestamp: None,
            emitted: std::collections::VecDeque::with_capacity(256),
            keyframe: false,
            vcl: false,
            ready: false,
            needs_keyframe: true,
            discontinuity: true,
            dropped: 0,
        }
    }
}
impl H264Assembler {
    fn clear_frame(&mut self) {
        self.buffer.clear();
        self.fragment = None;
        self.keyframe = false;
        self.vcl = false;
        self.ready = false;
    }
    fn lost(&mut self) {
        self.clear_frame();
        self.needs_keyframe = true;
        self.discontinuity = true;
        self.damaged_timestamp = self.timestamp;
        self.dropped = self.dropped.saturating_add(1);
    }
    fn append(&mut self, data: &[u8]) -> Result<(), TransportError> {
        if data.len() > MAX_ACCESS_UNIT.saturating_sub(self.buffer.len()) {
            return Err(TransportError::Oversized);
        }
        self.buffer.extend_from_slice(data);
        Ok(())
    }
    fn nal(&mut self, nal: &[u8]) -> Result<(), TransportError> {
        let first = *nal.first().ok_or(TransportError::Malformed)?;
        let kind = first & 31;
        if first & 0x80 != 0 || !(1..=23).contains(&kind) {
            return Err(TransportError::Malformed);
        }
        if kind == 7 {
            if nal.len() > 65535 {
                return Err(TransportError::Oversized);
            }
            if self.sps != nal {
                self.sps.clear();
                self.sps.extend_from_slice(nal);
            }
        }
        if kind == 8 {
            if nal.len() > 65535 {
                return Err(TransportError::Oversized);
            }
            if self.pps != nal {
                self.pps.clear();
                self.pps.extend_from_slice(nal);
            }
        }
        self.vcl |= matches!(kind, 1..=5);
        self.keyframe |= kind == 5;
        self.append(&(nal.len() as u32).to_be_bytes())?;
        self.append(nal)
    }
    pub fn push(&mut self, bytes: &[u8]) -> Result<Option<EncodedFrame<'_>>, TransportError> {
        let packet = match Rtp::parse(bytes) {
            Ok(p) => p,
            Err(e) => {
                self.lost();
                return Err(e);
            }
        };
        // The private receiver pipeline explicitly negotiates dynamic type 96.
        if packet.payload_type != 96 {
            return Err(TransportError::Unsupported);
        }
        if self.ready {
            self.clear_frame();
        }
        if self.ssrc.is_some_and(|s| s != packet.ssrc) {
            self.lost();
            self.sequence = None;
            self.timestamp = None;
            self.sps.clear();
            self.pps.clear();
            self.damaged_timestamp = None;
            self.emitted.clear();
        }
        self.ssrc = Some(packet.ssrc);
        if self.sequence == Some(packet.sequence) {
            return Ok(None);
        }
        if self
            .sequence
            .is_some_and(|s| s.wrapping_add(1) != packet.sequence)
        {
            self.lost();
            self.damaged_timestamp = Some(packet.timestamp);
            self.sequence = Some(packet.sequence);
            self.timestamp = Some(packet.timestamp);
            return Err(TransportError::PacketLoss);
        }
        self.sequence = Some(packet.sequence);
        if self.emitted.contains(&packet.timestamp)
            || self.damaged_timestamp == Some(packet.timestamp)
        {
            return Ok(None);
        }
        if self.timestamp.is_some_and(|t| t != packet.timestamp) && !self.buffer.is_empty() {
            self.lost();
        }
        self.timestamp = Some(packet.timestamp);
        let result = self.payload(packet.payload);
        if let Err(error) = result {
            self.lost();
            return Err(error);
        }
        if !packet.marker {
            return Ok(None);
        }
        if self.fragment.is_some() {
            self.lost();
            return Err(TransportError::PacketLoss);
        }
        if !self.vcl {
            self.clear_frame();
            return Ok(None);
        }
        if self.needs_keyframe && !self.keyframe {
            self.clear_frame();
            self.damaged_timestamp = self.timestamp;
            self.dropped = self.dropped.saturating_add(1);
            return Ok(None);
        }
        self.needs_keyframe = false;
        self.ready = true;
        if self.emitted.len() == 256 {
            self.emitted.pop_front();
        }
        self.emitted.push_back(packet.timestamp);
        let discontinuity = std::mem::replace(&mut self.discontinuity, false);
        Ok(Some(EncodedFrame {
            avcc: &self.buffer,
            sps: &self.sps,
            pps: &self.pps,
            timestamp_90khz: packet.timestamp,
            keyframe: self.keyframe,
            discontinuity,
        }))
    }
    fn payload(&mut self, payload: &[u8]) -> Result<(), TransportError> {
        if payload[0] & 0x80 != 0 {
            return Err(TransportError::Malformed);
        }
        match payload[0] & 31 {
            1..=23 => {
                if self.fragment.is_some() {
                    return Err(TransportError::PacketLoss);
                }
                self.nal(payload)
            }
            24 => {
                if self.fragment.is_some() {
                    return Err(TransportError::PacketLoss);
                }
                let mut offset = 1;
                let mut count = 0;
                while offset < payload.len() {
                    let len = payload
                        .get(offset..offset + 2)
                        .ok_or(TransportError::Malformed)?;
                    let len = u16::from_be_bytes([len[0], len[1]]) as usize;
                    offset += 2;
                    if len == 0 || count >= 64 {
                        return Err(TransportError::Malformed);
                    }
                    let nal = payload
                        .get(offset..offset + len)
                        .ok_or(TransportError::Malformed)?;
                    self.nal(nal)?;
                    offset += len;
                    count += 1;
                }
                if count == 0 {
                    return Err(TransportError::Malformed);
                }
                Ok(())
            }
            28 => {
                if payload.len() < 3 {
                    return Err(TransportError::Malformed);
                }
                let header = payload[1];
                let start = header & 0x80 != 0;
                let end = header & 0x40 != 0;
                let kind = header & 31;
                if header & 0x20 != 0 || (start && end) || !(1..=23).contains(&kind) {
                    return Err(TransportError::Malformed);
                }
                if start {
                    if self.fragment.is_some() {
                        return Err(TransportError::PacketLoss);
                    }
                    let offset = self.buffer.len();
                    self.append(&[0; 4])?;
                    self.append(&[(payload[0] & 0xe0) | kind])?;
                    self.fragment = Some(offset);
                    self.keyframe |= kind == 5;
                    self.vcl |= matches!(kind, 1..=5);
                } else {
                    let offset = self.fragment.ok_or(TransportError::PacketLoss)?;
                    if self.buffer[offset + 4] != ((payload[0] & 0xe0) | kind) {
                        return Err(TransportError::Malformed);
                    }
                }
                if matches!(kind, 7 | 8) {
                    let offset = self.fragment.ok_or(TransportError::PacketLoss)?;
                    if self.buffer.len() - offset - 4 + payload.len() - 2 > 65535 {
                        return Err(TransportError::Oversized);
                    }
                }
                self.append(&payload[2..])?;
                if end {
                    let offset = self.fragment.take().ok_or(TransportError::PacketLoss)?;
                    let length = (self.buffer.len() - offset - 4) as u32;
                    self.buffer[offset..offset + 4].copy_from_slice(&length.to_be_bytes());
                    if matches!(kind, 7 | 8) {
                        let parameter = if kind == 7 {
                            &mut self.sps
                        } else {
                            &mut self.pps
                        };
                        parameter.clear();
                        parameter.extend_from_slice(&self.buffer[offset + 4..]);
                    }
                }
                Ok(())
            }
            _ => Err(TransportError::Unsupported),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn packet(seq: u16, ts: u32, marker: bool, payload: &[u8]) -> Vec<u8> {
        let mut p = vec![0x80, 96 | if marker { 128 } else { 0 }];
        p.extend_from_slice(&seq.to_be_bytes());
        p.extend_from_slice(&ts.to_be_bytes());
        p.extend_from_slice(&1u32.to_be_bytes());
        p.extend_from_slice(payload);
        p
    }
    #[test]
    fn fragmented_idr_and_sequence_wrap() -> Result<(), TransportError> {
        let mut p = H264Assembler::default();
        assert!(
            p.push(&packet(65535, 10, false, &[0x7c, 0x85, 1, 2]))?
                .is_none()
        );
        let f = p
            .push(&packet(0, 10, true, &[0x7c, 0x45, 3, 4]))?
            .ok_or(TransportError::Malformed)?;
        assert_eq!(f.avcc, &[0, 0, 0, 5, 0x65, 1, 2, 3, 4]);
        assert!(f.keyframe && f.discontinuity);
        Ok(())
    }
    #[test]
    fn loss_waits_for_keyframe() -> Result<(), TransportError> {
        let mut p = H264Assembler::default();
        p.push(&packet(1, 10, false, &[0x7c, 0x85, 1]))?;
        assert_eq!(
            p.push(&packet(3, 10, true, &[0x7c, 0x45, 3])).err(),
            Some(TransportError::PacketLoss)
        );
        assert!(p.push(&packet(4, 20, true, &[0x41, 1]))?.is_none());
        assert!(p.push(&packet(5, 30, true, &[0x65, 1]))?.is_some());
        Ok(())
    }
    #[test]
    fn aggregation_and_duplicate_packets() -> Result<(), TransportError> {
        let mut p = H264Assembler::default();
        let bytes = packet(
            1,
            10,
            true,
            &[24, 0, 2, 0x67, 1, 0, 2, 0x68, 2, 0, 2, 0x65, 3],
        );
        let f = p.push(&bytes)?.ok_or(TransportError::Malformed)?;
        assert_eq!(f.sps, &[0x67, 1]);
        assert_eq!(f.pps, &[0x68, 2]);
        assert!(p.push(&bytes)?.is_none());
        Ok(())
    }
    #[test]
    fn malformed_headers_extensions_padding_and_nals() {
        for size in 0..12 {
            assert!(Rtp::parse(&vec![0; size]).is_err());
        }
        let mut p = packet(1, 1, true, &[0x65, 1]);
        p[0] |= 0x10;
        assert!(Rtp::parse(&p).is_err());
        let mut p = packet(1, 1, true, &[0x65, 0]);
        p[0] |= 0x20;
        assert!(Rtp::parse(&p).is_err());
        let mut a = H264Assembler::default();
        assert!(a.push(&packet(1, 1, true, &[24, 255, 255, 0x65])).is_err());
        assert!(a.push(&packet(2, 2, true, &[0xe5, 0])).is_err());
        assert!(h264_dimensions(&[0x67, 0, 0, 0]).is_err());
    }
    #[test]
    fn fragmented_input_memory_is_bounded() {
        let mut a = H264Assembler::default();
        let _ = a.push(&packet(1, 1, false, &[0x7c, 0x85, 0]));
        let mut payload = vec![1u8; 65500];
        payload[0] = 0x7c;
        payload[1] = 5;
        let mut oversized = false;
        for sequence in 2..200 {
            if a.push(&packet(sequence, 1, false, &payload)).is_err() {
                oversized = true;
                break;
            }
        }
        assert!(oversized);
        assert!(a.buffer.len() <= MAX_ACCESS_UNIT);
        assert!(a.buffer.capacity() <= MAX_ACCESS_UNIT * 2);
    }

    #[test]
    fn loss_discards_the_entire_damaged_access_unit() -> Result<(), TransportError> {
        let mut p = H264Assembler::default();
        p.push(&packet(1, 10, false, &[0x65, 1]))?;
        assert_eq!(
            p.push(&packet(3, 10, false, &[0x65, 3])).err(),
            Some(TransportError::PacketLoss)
        );
        assert!(p.push(&packet(4, 10, true, &[0x65, 4]))?.is_none());
        assert!(p.push(&packet(5, 20, true, &[0x41, 5]))?.is_none());
        let recovered = p
            .push(&packet(6, 30, true, &[0x65, 6]))?
            .ok_or(TransportError::Malformed)?;
        assert!(recovered.discontinuity);
        Ok(())
    }

    #[test]
    fn malformed_fragment_cannot_leave_a_decodable_tail() -> Result<(), TransportError> {
        let mut p = H264Assembler::default();
        p.push(&packet(1, 10, false, &[0x7c, 0x85, 1]))?;
        assert_eq!(
            p.push(&packet(2, 10, false, &[0x7c, 0x41, 2])).err(),
            Some(TransportError::Malformed)
        );
        assert!(p.push(&packet(3, 10, true, &[0x65, 3]))?.is_none());
        assert!(p.push(&packet(4, 20, true, &[0x65, 4]))?.is_some());
        Ok(())
    }

    #[test]
    fn fragmented_parameter_sets_are_retained() -> Result<(), TransportError> {
        let mut p = H264Assembler::default();
        p.push(&packet(1, 10, false, &[0x7c, 0x87, 1, 2]))?;
        p.push(&packet(2, 10, false, &[0x7c, 0x47, 3, 4]))?;
        p.push(&packet(3, 10, false, &[0x7c, 0x88, 5]))?;
        p.push(&packet(4, 10, false, &[0x7c, 0x48, 6]))?;
        let frame = p
            .push(&packet(5, 10, true, &[0x65, 7]))?
            .ok_or(TransportError::Malformed)?;
        assert_eq!(frame.sps, &[0x67, 1, 2, 3, 4]);
        assert_eq!(frame.pps, &[0x68, 5, 6]);
        Ok(())
    }

    #[test]
    fn source_timestamps_are_unique_with_bounded_history() -> Result<(), TransportError> {
        let mut p = H264Assembler::default();
        for i in 0..300 {
            assert!(
                p.push(&packet(i, u32::from(i), true, &[0x65, 1]))?
                    .is_some()
            );
        }
        assert_eq!(p.emitted.len(), 256);
        assert!(p.push(&packet(300, 299, true, &[0x65, 1]))?.is_none());
        assert!(p.push(&packet(301, 300, true, &[0x41, 1]))?.is_some());
        Ok(())
    }

    #[test]
    fn unrelated_payload_type_does_not_damage_a_video_fragment() -> Result<(), TransportError> {
        let mut p = H264Assembler::default();
        p.push(&packet(1, 10, false, &[0x7c, 0x85, 1]))?;
        let mut other = packet(9, 20, true, &[0x65, 8]);
        other[1] = 97;
        assert_eq!(p.push(&other).err(), Some(TransportError::Unsupported));
        assert!(p.push(&packet(2, 10, true, &[0x7c, 0x45, 2]))?.is_some());
        Ok(())
    }
}
