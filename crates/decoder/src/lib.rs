use imirror_native_core::{NativeError, Session};
use imirror_video_core::{EncodedFrame, EncodedVideoSink, TransportError, h264_dimensions};
use std::sync::Arc;
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("Waiting for video parameter sets")]
    MissingConfiguration,
    #[error(transparent)]
    Format(#[from] TransportError),
    #[error(transparent)]
    Native(#[from] NativeError),
}
pub struct MediaFoundationSink {
    session: Arc<Session>,
    sps: Vec<u8>,
    width: u32,
    height: u32,
    last_timestamp: Option<u32>,
    extended: i64,
}
impl MediaFoundationSink {
    pub fn new(session: Arc<Session>) -> Self {
        Self {
            session,
            sps: Vec::new(),
            width: 0,
            height: 0,
            last_timestamp: None,
            extended: 90000,
        }
    }
}
impl EncodedVideoSink for MediaFoundationSink {
    type Error = DecodeError;
    fn submit(&mut self, frame: &EncodedFrame<'_>) -> Result<(), DecodeError> {
        if frame.sps.is_empty() || frame.pps.is_empty() {
            return Err(DecodeError::MissingConfiguration);
        }
        if frame.sps != self.sps {
            (self.width, self.height) = h264_dimensions(frame.sps)?;
            self.sps.clear();
            self.sps.extend_from_slice(frame.sps);
        }
        if frame.discontinuity {
            self.last_timestamp = None;
            self.extended = 90000;
        }
        if let Some(last) = self.last_timestamp {
            self.extended = self
                .extended
                .saturating_add(frame.timestamp_90khz.wrapping_sub(last) as i32 as i64);
        }
        self.last_timestamp = Some(frame.timestamp_90khz);
        let ticks =
            ((self.extended.max(0) as i128 * 10_000_000) / 90_000).min(i64::MAX as i128) as i64;
        self.session.submit_encoded(
            frame.avcc,
            frame.sps,
            frame.pps,
            self.width,
            self.height,
            ticks,
            frame.keyframe,
            frame.discontinuity,
        )?;
        Ok(())
    }
}
