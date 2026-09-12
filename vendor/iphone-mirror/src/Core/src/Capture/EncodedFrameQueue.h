// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include <array>
#include <chrono>
#include <cstdint>
#include <span>
#include <utility>
#include <vector>

namespace iPhoneMirror::capture {
struct EncodedPacket {
    std::vector<std::uint8_t> avcc, sps, pps;
    std::uint32_t width{}, height{};
    std::int64_t pts{};
    bool keyframe{}, discontinuity{};
    std::uint64_t generation{};
    std::chrono::steady_clock::time_point received;
    std::size_t bytes() const noexcept { return avcc.size()+sps.size()+pps.size(); }
};

// Caller serializes access. Compressed dependent frames cannot be dropped like
// decoded textures. Allow bounded headroom while MF initializes and for network
// bursts afterward. There is no minimum fill or intentional playout delay.
class EncodedFrameQueue {
public:
    using Clock = std::chrono::steady_clock;
    static constexpr std::size_t StartupPackets = 32;
    static constexpr std::size_t StreamingPackets = 16;
    static constexpr std::size_t MaxBytes = 16U*1024U*1024U;
    static constexpr auto StartupAge = std::chrono::milliseconds(500);
    static constexpr auto StreamingAge = std::chrono::milliseconds(250);

    std::size_t size() const noexcept { return count_; }
    std::size_t bytes() const noexcept { return bytes_; }
    bool startup() const noexcept { return startup_; }
    auto max_age() const noexcept { return startup_ ? StartupAge : StreamingAge; }
    bool expired(Clock::time_point now) const noexcept {
        return count_ && now-packets_[head_].received > max_age();
    }
    bool can_push(std::size_t bytes, Clock::time_point now) const noexcept {
        return rejection(bytes,now)==nullptr;
    }
    const char* rejection(std::size_t bytes, Clock::time_point now) const noexcept {
        if(expired(now)) return "encoded_age_limit";
        if(count_>=limit()) return "encoded_packet_limit";
        if(bytes>MaxBytes-bytes_) return "encoded_byte_limit";
        return nullptr;
    }
    template<class Fill>
    bool push(std::size_t bytes, Clock::time_point now, Fill&& fill) {
        if (!can_push(bytes,now)) return false;
        auto& packet=packets_[(head_+count_)%limit()];
        fill(packet);
        bytes_+=packet.bytes(); ++count_;
        return true;
    }
    void pop(EncodedPacket& packet) {
        // Startup slots hold only pending payloads: don't retain a large consumed
        // frame in each of 32 slots. Normal streaming reuses its bounded pool.
        if(startup_) packet=EncodedPacket{};
        bytes_-=packets_[head_].bytes();
        std::swap(packet,packets_[head_]);
        head_=(head_+1)%limit(); --count_;
        finish_startup();
    }
    void decoded() { decoded_=true; finish_startup(); }
    void reset() {
        for(auto& packet:packets_) packet=EncodedPacket{};
        head_=count_=bytes_=0; startup_=true; decoded_=false;
    }
private:
    std::array<EncodedPacket,StartupPackets> packets_;
    std::size_t head_{},count_{},bytes_{};
    bool startup_{true},decoded_{};
    std::size_t limit() const noexcept { return startup_ ? StartupPackets : StreamingPackets; }
    void finish_startup() {
        if(!startup_ || !decoded_ || count_>StreamingPackets) return;
        std::array<EncodedPacket,StreamingPackets> pending;
        for(std::size_t i=0;i<count_;++i)
            std::swap(pending[i],packets_[(head_+i)%StartupPackets]);
        for(auto& packet:packets_) packet=EncodedPacket{};
        for(std::size_t i=0;i<count_;++i) std::swap(pending[i],packets_[i]);
        head_=0; startup_=false;
    }
};
}
