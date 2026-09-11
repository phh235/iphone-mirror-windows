// SPDX-License-Identifier: GPL-3.0-only
#pragma once
#include "Capture/ICaptureSession.h"
#include "Audio/WasapiRenderer.h"
#include <array>
#include <atomic>
#include <condition_variable>
#include <mutex>
#include <thread>

namespace iPhoneMirror::capture {
// Transport-independent encoded input. Decoder and renderer are the same native
// implementations used by USB; the producer never maps a decoded GPU surface.
class EncodedSession final : public ICaptureSession {
public:
    explicit EncodedSession(CapturePreferences preferences);
    ~EncodedSession() override;
    void submit(std::span<const std::uint8_t> avcc, std::span<const std::uint8_t> sps,
        std::span<const std::uint8_t> pps, std::uint32_t width, std::uint32_t height,
        std::int64_t pts, bool keyframe, bool discontinuity);
    void submit_audio(std::span<const std::uint8_t> pcm, std::uint32_t rate, std::uint32_t channels);
    void reset();
    void stop() noexcept override;
    Snapshot snapshot() const override;
    std::int64_t latest_frame_timestamp() const override;
    std::shared_ptr<const media::DecodedFrame> latest_frame() const override;
    std::shared_ptr<const media::DecodedFrame> next_render_frame() override { return latest_frame(); }
    std::shared_ptr<const AudioPacket> next_audio_packet(std::uint64_t) const override { return {}; }
    void set_audio_enabled(bool value) noexcept override;
    void set_audio_volume(float value) noexcept override;
    void set_target_fps(std::uint32_t value) noexcept override { fps_cap_.store(value); }
    std::uint32_t target_fps() const noexcept override { return fps_cap_.load(); }
    void set_decoder_preference(media::DecoderPreference value) noexcept override { decoder_preference_.store(value); }
    DecoderSwitchStatus decoder_switch_status() const noexcept override;
    void request_display_orientation(bool) noexcept override {}
private:
    struct Packet {
        std::vector<std::uint8_t> avcc, sps, pps;
        std::uint32_t width{}, height{};
        std::int64_t pts{};
        bool keyframe{}, discontinuity{};
        std::uint64_t generation{};
        std::chrono::steady_clock::time_point received;
    };
    mutable std::mutex state_mutex_, queue_mutex_, audio_mutex_;
    std::condition_variable_any available_;
    Snapshot status_;
    std::shared_ptr<const media::DecodedFrame> latest_;
    std::array<Packet,3> queue_;
    std::size_t head_{}, count_{};
    bool waiting_for_keyframe_{true};
    std::atomic_uint64_t generation_{1};
    std::atomic_uint32_t fps_cap_{0};
    std::atomic_bool audio_enabled_{true};
    std::atomic<float> volume_{1.0F};
    std::atomic<media::DecoderPreference> decoder_preference_{media::DecoderPreference::Auto};
    DecoderSwitchStatus decoder_status_;
    std::unique_ptr<audio::WasapiRenderer> audio_;
    std::uint32_t audio_rate_{}, audio_channels_{};
    std::uint64_t source_frames_{}, fps_previous_{};
    std::chrono::steady_clock::time_point fps_at_{std::chrono::steady_clock::now()};
    std::jthread worker_;
    void run(std::stop_token token) noexcept;
};
}
