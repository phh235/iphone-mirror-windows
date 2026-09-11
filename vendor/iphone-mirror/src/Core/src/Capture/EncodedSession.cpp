// SPDX-License-Identifier: GPL-3.0-only
#include "Capture/EncodedSession.h"
#include "Protocol/QuickTimePacket.h"
#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace iPhoneMirror::capture {
EncodedSession::EncodedSession(CapturePreferences preferences)
    : fps_cap_(preferences.target_fps), audio_enabled_(preferences.play_audio),
      volume_(preferences.audio_volume), decoder_preference_(preferences.decoder_preference) {
    status_.state=State::WaitingForDevice;
    status_.message=L"Open Screen Mirroring on your iPhone and select iMirror.";
    for(auto& packet:queue_) packet.avcc.reserve(256*1024);
    worker_=std::jthread([this](std::stop_token token){run(token);});
}
EncodedSession::~EncodedSession(){stop();}
void EncodedSession::stop() noexcept {
    worker_.request_stop(); available_.notify_all();
    if(worker_.joinable()) worker_.join();
    {std::scoped_lock lock(audio_mutex_);if(audio_) audio_->stop();audio_.reset();}
    std::scoped_lock lock(state_mutex_);latest_.reset();status_.state=State::Stopped;
}
void EncodedSession::reset() {
    {std::scoped_lock lock(queue_mutex_);count_=0;head_=0;waiting_for_keyframe_=true;++generation_;}
    {std::scoped_lock lock(state_mutex_);latest_.reset();status_.state=State::WaitingForDevice;
     status_.fps=0;status_.message=L"AirPlay disconnected. Select iMirror on your iPhone to reconnect.";}
    {std::scoped_lock lock(audio_mutex_);audio_.reset();audio_rate_=audio_channels_=0;}
}
void EncodedSession::submit(std::span<const std::uint8_t> avcc,std::span<const std::uint8_t> sps,
    std::span<const std::uint8_t> pps,std::uint32_t width,std::uint32_t height,
    std::int64_t pts,bool keyframe,bool discontinuity) {
    if(avcc.empty()||avcc.size()>8*1024*1024||sps.size()<4||sps.size()>65535||pps.empty()||pps.size()>65535||
       width==0||height==0||width>8192||height>8192||pts<0||
       (sps[0]&31)!=7||(pps[0]&31)!=8) throw std::invalid_argument("Invalid encoded video input");
    // Validate every AVCC range before handing device data to the system decoder.
    for(std::size_t offset=0;offset<avcc.size();) {
        if(avcc.size()-offset<4) throw std::invalid_argument("Truncated AVCC size");
        const auto length=(std::uint32_t(avcc[offset])<<24)|(std::uint32_t(avcc[offset+1])<<16)|
            (std::uint32_t(avcc[offset+2])<<8)|avcc[offset+3];
        offset+=4;
        if(length==0||length>avcc.size()-offset||(avcc[offset]&0x80)!=0)
            throw std::invalid_argument("Invalid AVCC NAL");
        offset+=length;
    }
    const auto now=std::chrono::steady_clock::now();
    {
        std::scoped_lock lock(state_mutex_);
        ++source_frames_;status_.video_frames=source_frames_;status_.width=width;status_.height=height;
        const auto seconds=std::chrono::duration<double>(now-fps_at_).count();
        if(seconds>=0.5){status_.fps=double(source_frames_-fps_previous_)/seconds;fps_previous_=source_frames_;fps_at_=now;}
    }
    {
        std::scoped_lock lock(queue_mutex_);
        if(discontinuity||count_==queue_.size()){
            count_=0;head_=0;waiting_for_keyframe_=true;++generation_;
        }
        if(waiting_for_keyframe_&&!keyframe) return;
        if(waiting_for_keyframe_){discontinuity=true;waiting_for_keyframe_=false;}
        auto& packet=queue_[(head_+count_)%queue_.size()];
        packet.avcc.assign(avcc.begin(),avcc.end());
        packet.sps.assign(sps.begin(),sps.end());packet.pps.assign(pps.begin(),pps.end());
        packet.width=width;packet.height=height;packet.pts=pts;
        packet.keyframe=keyframe;packet.discontinuity=discontinuity;
        packet.generation=generation_.load();packet.received=now;++count_;
    }
    available_.notify_one();
}
void EncodedSession::run(std::stop_token token) noexcept {
    Packet packet;packet.avcc.reserve(256*1024);
    std::unique_ptr<media::MediaFoundationVideoDecoder> decoder;
    std::vector<std::uint8_t> sps,pps;
    auto applied=media::DecoderPreference::Auto;
    auto retry_after=std::chrono::steady_clock::time_point{};
    std::uint64_t decoded_generation{};
    while(!token.stop_requested()) {
        {
            std::unique_lock lock(queue_mutex_);
            if(!available_.wait(lock,token,[this]{return count_!=0;})) break;
            std::swap(packet,queue_[head_]);head_=(head_+1)%queue_.size();--count_;
        }
        if(packet.generation!=generation_.load()||std::chrono::steady_clock::now()<retry_after) continue;
        try {
            const auto wanted=decoder_preference_.load();
            const bool configure=!decoder||sps!=packet.sps||pps!=packet.pps||
                (packet.keyframe&&wanted!=applied);
            if(configure){
                if(!packet.keyframe) continue;
                decoder=std::make_unique<media::MediaFoundationVideoDecoder>(wanted);
                coremedia::FormatDescription format;
                format.media_type=quicktime::fourcc('v','i','d','e');
                format.codec=quicktime::fourcc('a','v','c','1');
                format.width=packet.width;format.height=packet.height;
                format.sequence_parameter_sets={packet.sps};format.picture_parameter_sets={packet.pps};
                format.nalu_length_size=4;
                decoder->configure(format);
                sps=packet.sps;pps=packet.pps;applied=wanted;
                std::scoped_lock lock(state_mutex_);
                decoder_status_.requested=decoder_status_.applied=wanted;
                decoder_status_.runtime_mode=decoder->selected_decoder_is_hardware()
                    ? DecoderRuntimeMode::Hardware:DecoderRuntimeMode::Software;
            }
            if(packet.discontinuity||decoded_generation!=packet.generation) decoder->flush();
            decoded_generation=packet.generation;
            const auto start=std::chrono::steady_clock::now();
            auto frames=decoder->decode(packet.avcc,packet.pts,166667);
            const auto elapsed=std::chrono::duration<double,std::milli>(std::chrono::steady_clock::now()-start).count();
            if(packet.generation!=generation_.load()) continue;
            for(auto& frame:frames) {
                frame.received_at=packet.received;
                auto latest=std::make_shared<const media::DecodedFrame>(std::move(frame));
                std::scoped_lock lock(state_mutex_);
                latest_=std::move(latest);status_.state=State::Streaming;
                status_.latency_ms=elapsed;status_.error_code=0;status_.failure_kind=FailureKind::None;
                status_.message=L"AirPlay mirroring";
            }
        } catch(...) {
            decoder.reset();retry_after=std::chrono::steady_clock::now()+std::chrono::seconds(1);
            {std::scoped_lock lock(queue_mutex_);count_=0;head_=0;waiting_for_keyframe_=true;++generation_;}
            std::scoped_lock lock(state_mutex_);
            status_.state=State::Handshaking;status_.failure_kind=FailureKind::VideoStream;
            status_.failure_stage=FailureStage::Decoder;status_.error_code=-3001;
            status_.message=L"Decoder reset. Waiting for a new AirPlay keyframe.";
        }
    }
}
void EncodedSession::submit_audio(std::span<const std::uint8_t> pcm,std::uint32_t rate,std::uint32_t channels) {
    if((rate!=44100&&rate!=48000)||channels==0||channels>2||pcm.empty()||pcm.size()>65536||
       pcm.size()%(channels*2)!=0) throw std::invalid_argument("Invalid PCM packet");
    std::scoped_lock lock(audio_mutex_);
    if(!audio_||rate!=audio_rate_||channels!=audio_channels_) {
        coremedia::AudioStreamBasicDescription format{double(rate),quicktime::fourcc('l','p','c','m'),
            12,channels*2,1,channels*2,channels,16,0};
        audio_=std::make_unique<audio::WasapiRenderer>(format,audio_enabled_.load(),volume_.load(),
            audio::WasapiBufferingMode::NetworkJitter);
        audio_rate_=rate;audio_channels_=channels;
    }
    audio_->enqueue(pcm);
    std::scoped_lock state_lock(state_mutex_);
    ++status_.audio_packets;status_.audio_sample_rate=rate;status_.audio_channels=channels;
}
Snapshot EncodedSession::snapshot() const {std::scoped_lock lock(state_mutex_);return status_;}
std::int64_t EncodedSession::latest_frame_timestamp() const {
    std::scoped_lock lock(state_mutex_);return latest_?latest_->timestamp_100ns:0;
}
std::shared_ptr<const media::DecodedFrame> EncodedSession::latest_frame() const {
    std::scoped_lock lock(state_mutex_);return latest_;
}
void EncodedSession::set_audio_enabled(bool value) noexcept {
    audio_enabled_.store(value);std::scoped_lock lock(audio_mutex_);if(audio_)audio_->set_enabled(value);
}
void EncodedSession::set_audio_volume(float value) noexcept {
    volume_.store(value);std::scoped_lock lock(audio_mutex_);if(audio_)audio_->set_volume(value);
}
DecoderSwitchStatus EncodedSession::decoder_switch_status() const noexcept {
    std::scoped_lock lock(state_mutex_);auto result=decoder_status_;
    result.requested=decoder_preference_.load();
    result.phase=result.requested==result.applied?DecoderSwitchPhase::Applied:DecoderSwitchPhase::Pending;
    return result;
}
}
