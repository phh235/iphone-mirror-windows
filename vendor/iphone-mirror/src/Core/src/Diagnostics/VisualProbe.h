// SPDX-License-Identifier: GPL-3.0-only
#pragma once
// Opt-in benchmark ROI observations; never modifies or materializes video pixels.
#include "Media/MediaFoundationDecoder.h"
#include <Windows.h>
#include <algorithm>
#include <array>
#include <atomic>
#include <cmath>
#include <mutex>
#include <limits>

namespace iPhoneMirror::diagnostics::visual {
struct Observation {
    std::int64_t pts{}, present_start{}, present_end{};
    std::uint64_t swap_chain{};
    std::uint32_t changed_samples{}, reserved{};
};
struct State {
    std::atomic_bool enabled{false};
    std::mutex mutex;
    std::uintptr_t window{};
    double left{},top{},width{},height{};
    std::array<std::uint8_t,256> latest{}, baseline{}, reference{};
    std::array<Observation,256> observations{};
    std::uint64_t generation{};
    std::size_t count{};
    bool available{}, armed{}, overflow{}, reference_valid{};
    std::uint32_t source_width{},source_height{};
};
inline State state;
inline int configure(std::uintptr_t window,double left,double top,double width,double height) {
    if(!window || !std::isfinite(left) || !std::isfinite(top) || !std::isfinite(width) ||
        !std::isfinite(height) || left<0 || top<0 || width<=0 || height<=0 ||
        left+width>1 || top+height>1) return -1;
    std::scoped_lock lock(state.mutex);
    state.window=window;state.left=left;state.top=top;state.width=width;state.height=height;
    state.available=false;state.armed=false;state.count=0;state.overflow=false;
    state.reference_valid=false;
    state.source_width=state.source_height=0;
    state.enabled.store(true,std::memory_order_release);
    return 0;
}
inline std::uint64_t arm() {
    std::scoped_lock lock(state.mutex);
    if(!state.enabled.load() || !state.available) return 0;
    std::size_t changed=0;
    for(std::size_t i=0;i<state.latest.size();++i)
        changed+=std::abs(int(state.latest[i])-int(state.reference[i]))>=24;
    if(!state.reference_valid || changed>=8) return std::numeric_limits<std::uint64_t>::max();
    state.baseline=state.latest;state.count=0;state.overflow=false;state.armed=true;
    if(++state.generation==0 || state.generation==std::numeric_limits<std::uint64_t>::max()) state.generation=1;
    return state.generation;
}
inline void disable() {
    state.enabled.store(false,std::memory_order_release);
    std::scoped_lock lock(state.mutex);
    state.armed=false;state.available=false;
}
inline int read(std::uint64_t generation,Observation* output,std::uint32_t capacity) {
    if(!output || capacity<state.observations.size()) return -1;
    std::scoped_lock lock(state.mutex);
    if(generation!=state.generation || !state.available) return -2;
    if(state.overflow) return -3;
    std::copy_n(state.observations.begin(),state.count,output);
    return static_cast<int>(state.count);
}
inline void observe(HWND window,const media::DecodedFrame& frame,std::int64_t start,
    std::int64_t end,std::uintptr_t swap_chain,std::uint32_t render_width,std::uint32_t render_height) {
    if(!state.enabled.load(std::memory_order_acquire)) return;
    std::scoped_lock lock(state.mutex);
    if(reinterpret_cast<std::uintptr_t>(window)!=state.window) return;
    if(render_width<32 || render_height<32 || render_width>65535 || render_height>65535 ||
        frame.pixel_format!=media::PixelFormat::Nv12 || frame.stride<=0 || frame.width==0 || frame.height==0 ||
        static_cast<std::uint32_t>(frame.stride)<frame.width ||
        frame.nv12.size()<static_cast<std::size_t>(frame.stride)*frame.height) {
        state.available=false;state.armed=false;return;
    }
    if(state.source_width && (state.source_width!=frame.width || state.source_height!=frame.height)) {
        state.available=false;state.armed=false;return;
    }
    state.source_width=frame.width;state.source_height=frame.height;
    std::uint32_t changed=0;
    for(std::size_t y=0;y<16;++y) for(std::size_t x=0;x<16;++x) {
        const auto px=std::min(frame.width-1,static_cast<std::uint32_t>((state.left+(double(x)+0.5)*state.width/16)*frame.width));
        const auto py=std::min(frame.height-1,static_cast<std::uint32_t>((state.top+(double(y)+0.5)*state.height/16)*frame.height));
        const auto value=frame.nv12[static_cast<std::size_t>(py)*frame.stride+px];
        const auto index=y*16+x;
        state.latest[index]=value;
        changed+=std::abs(int(value)-int(state.baseline[index]))>=24;
    }
    state.available=true;
    if(!state.reference_valid) {state.reference=state.latest;state.reference_valid=true;}
    if(state.armed && changed>=8) {
        if(state.count==state.observations.size()) {state.overflow=true;return;}
        state.observations[state.count++]={frame.timestamp_100ns,start,end,swap_chain,changed,
            (render_width<<16U)|render_height};
    }
}
} // namespace iPhoneMirror::diagnostics::visual
