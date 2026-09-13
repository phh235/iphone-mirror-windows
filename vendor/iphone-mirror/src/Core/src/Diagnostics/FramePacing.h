// SPDX-License-Identifier: GPL-3.0-only
#pragma once
// Opt-in observation only. No frame ownership, queue, decoder or Present policy.
#include <Windows.h>
#include <algorithm>
#include <atomic>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <thread>
#include <vector>

namespace iPhoneMirror::diagnostics::pacing {
enum Kind : std::uint32_t { Source=1, Decode=2, Publish=3, Present=4,
    Render=5, Queue=6, Unclocked=7, DxgiOutput=8, StagingCreate=9,
    GpuCopy=10, ReadbackMap=11, CpuAllocate=12, CpuCopy=13, ProcessOutput=14,
    Upload=15, RenderStart=16, DecodeStart=17, UsbRead=18, PacketBatch=19,
    AnnexB=20, DecoderInputBuffer=21, ProcessInput=22, GpuShare=23,
    GpuConsumer=24, GpuImport=25, GpuMutex=26, GpuRelease=27, Probe=99 };
struct Event {
    std::int64_t qpc{}, pts{}, a{}, b{}, c{};
    std::uint32_t kind{};
};
inline constexpr std::size_t Capacity=262144;
struct Recorder {
    std::atomic_uint64_t epoch{0}, writers{0}, next{0};
    std::vector<Event> events;
    std::int64_t started{}, stopped{}, frequency{};
};
inline Recorder recorder;
inline std::int64_t qpc() noexcept {
    LARGE_INTEGER value{}; QueryPerformanceCounter(&value); return value.QuadPart;
}
inline std::int64_t stamp() noexcept {
    return recorder.epoch.load(std::memory_order_relaxed)&1 ? qpc() : 0;
}
inline void record(Kind kind, std::int64_t pts, std::int64_t a=0,
    std::int64_t b=0, std::int64_t c=0) noexcept {
    const auto epoch=recorder.epoch.load(std::memory_order_acquire);
    if(!(epoch&1)) return;
    recorder.writers.fetch_add(1,std::memory_order_acq_rel);
    if(recorder.epoch.load(std::memory_order_acquire)==epoch) {
        const auto index=recorder.next.fetch_add(1,std::memory_order_relaxed);
        if(index<Capacity) recorder.events[index]={qpc(),pts,a,b,c,kind};
    }
    recorder.writers.fetch_sub(1,std::memory_order_release);
}
// Span a=start QPC; event QPC=end; b=bytes/result. Never synchronizes the GPU.
inline void span(Kind kind, std::int64_t pts, std::int64_t start,
    std::int64_t detail=0) noexcept {
    if(start) record(kind,pts,start,detail);
}
inline void disable() noexcept {
    const auto epoch=recorder.epoch.load(std::memory_order_acquire);
    if(epoch&1) recorder.epoch.fetch_add(1,std::memory_order_acq_rel);
    while(recorder.writers.load(std::memory_order_acquire)!=0) std::this_thread::yield();
    recorder.stopped=qpc();
}
// Begin/end are serialized by the benchmark UI. An old writer's epoch check
// prevents it from touching a new capture's storage after disable()/begin().
inline std::int64_t begin() {
    disable();
    recorder.events.resize(Capacity); // Allocate once, never in frame callbacks.
    recorder.next.store(0,std::memory_order_relaxed);
    LARGE_INTEGER frequency{}; QueryPerformanceFrequency(&frequency);
    recorder.frequency=frequency.QuadPart;
    recorder.started=qpc();
    recorder.epoch.fetch_add(1,std::memory_order_release);
    return recorder.started;
}
inline bool finish(const wchar_t* path) {
    disable();
    std::ofstream output(std::filesystem::path(path),std::ios::trunc);
    if(!output) return false;
    const auto count=recorder.next.load(std::memory_order_acquire);
    output << "qpc,kind,pts,a,b,c\n";
    output << recorder.started << ",0,0," << recorder.frequency << ',' << Capacity << ",0\n";
    for(std::size_t i=0;i<std::min<std::uint64_t>(count,Capacity);++i) {
        const auto& e=recorder.events[i];
        output << e.qpc << ',' << e.kind << ',' << e.pts << ',' << e.a << ',' << e.b << ',' << e.c << '\n';
    }
    output << recorder.stopped << ",100,0," << count << ',' << (count>Capacity?count-Capacity:0) << ",0\n";
    return output.good();
}
inline double overhead_ns() {
    begin();
    const auto start=qpc();
    for(std::int64_t i=0;i<50000;++i) record(Probe,i);
    const auto elapsed=qpc()-start;
    disable();
    return double(elapsed)*1e9/double(recorder.frequency)/50000.0;
}
} // namespace iPhoneMirror::diagnostics::pacing
