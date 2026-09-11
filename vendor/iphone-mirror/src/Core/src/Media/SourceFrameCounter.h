// SPDX-License-Identifier: GPL-3.0-only
// iMirror addition: counts timestamped source samples, never UI refreshes.
#pragma once
#include "Media/CoreMedia.h"
#include <array>
#include <algorithm>
#include <limits>
#include <optional>

namespace iPhoneMirror::media {
class SourceFrameCounter {
public:
    void reset() noexcept { *this = {}; }
    void observe(const coremedia::SampleBuffer& sample) noexcept {
        if (sample.sample_data.empty() || sample.sample_count == 0) return;
        if (sample.sample_count > 1024) { ++unclocked_; return; }
        for (std::uint32_t i = 0; i < sample.sample_count; ++i) {
            std::optional<std::int64_t> pts;
            std::int64_t epoch{};
            if (sample.timing.size() == sample.sample_count) {
                pts = sample.timing[i].presentation_timestamp.to_100ns();
                epoch = sample.timing[i].presentation_timestamp.epoch;
            } else {
                std::optional<coremedia::CMTime> base = sample.output_presentation_timestamp;
                if (!base && !sample.timing.empty()) base = sample.timing.front().presentation_timestamp;
                if (base) { pts = base->to_100ns(); epoch = base->epoch; }
                if (i != 0) {
                    const auto duration = sample.timing.empty() ? std::nullopt : sample.timing.front().duration.to_100ns();
                    if (!pts || !duration || *duration <= 0 ||
                        *duration > std::numeric_limits<std::int64_t>::max() / i) {
                        pts.reset();
                    } else {
                        const auto offset = *duration * i;
                        if (*pts > std::numeric_limits<std::int64_t>::max() - offset) pts.reset();
                        else *pts += offset;
                    }
                }
            }
            if (!pts) { ++unclocked_; continue; }
            const Key key{epoch, *pts};
            if (std::find(recent_.begin(), recent_.begin() + size_, key) != recent_.begin() + size_) {
                ++duplicates_;
                continue;
            }
            recent_[next_] = key;
            next_ = (next_ + 1) % recent_.size();
            size_ = std::min(size_ + 1, recent_.size());
            ++unique_;
        }
    }
    [[nodiscard]] std::uint64_t unique() const noexcept { return unique_; }
    [[nodiscard]] std::uint64_t duplicates() const noexcept { return duplicates_; }
    [[nodiscard]] bool timing_complete() const noexcept { return unclocked_ == 0; }
private:
    struct Key { std::int64_t epoch{}, pts{}; bool operator==(const Key&) const = default; };
    std::array<Key, 256> recent_{};
    std::size_t size_{}, next_{};
    std::uint64_t unique_{}, duplicates_{}, unclocked_{};
};
}
