// SPDX-License-Identifier: GPL-3.0-only
#include "Capture/EncodedFrameQueue.h"
#include <iostream>
#include <stdexcept>

using namespace iPhoneMirror::capture;
using namespace std::chrono_literals;
namespace {
void check(bool value,const char* message) {
    if(!value) throw std::runtime_error(message);
}
bool push(EncodedFrameQueue& queue,int number,EncodedFrameQueue::Clock::time_point at,
    std::size_t bytes=1024) {
    return queue.push(bytes,at,[&](EncodedPacket& packet){
        packet.avcc.assign(bytes,0);packet.sps.clear();packet.pps.clear();
        packet.pts=number;packet.keyframe=number==0;packet.received=at;
    });
}
void delayed_decoder_and_drain() {
    EncodedFrameQueue queue;
    const auto start=EncodedFrameQueue::Clock::time_point{};
    check(push(queue,0,start),"first keyframe accepted");
    EncodedPacket in_flight;
    queue.pop(in_flight); // Decoder now holds the initial keyframe.
    for(int i=1;i<=20;++i)
        check(push(queue,i,start+16ms*i),"preserve dependencies while decoder is stalled");
    check(queue.startup() && queue.size()==20,"startup backlog retained beyond streaming limit");
    check(in_flight.keyframe && in_flight.pts==0,"initial keyframe still valid");
    queue.decoded(); // Decoder publishes that keyframe.
    check(queue.startup(),"do not truncate a startup backlog on first decode");
    for(int i=1;i<=20;++i) {
        queue.pop(in_flight);
        check(in_flight.pts==i,"ordered drain preserves compressed reference chain");
        if(queue.size()>EncodedFrameQueue::StreamingPackets) check(queue.startup(),"retain startup limit while draining");
    }
    check(!queue.startup() && queue.bytes()==0,"restore streaming limit after catch-up");
    for(int i=0;i<16;++i) check(push(queue,i,start),"bounded network burst accepted");
    check(!push(queue,16,start),"streaming queue remains bounded");
    for(int i=0;i<16;++i) {queue.pop(in_flight);check(in_flight.pts==i,"burst drains in source order");}
}
void bounds_and_reset() {
    EncodedFrameQueue queue;EncodedPacket packet;
    const auto now=EncodedFrameQueue::Clock::time_point{};
    for(std::size_t i=0;i<EncodedFrameQueue::StartupPackets;++i)
        check(push(queue,static_cast<int>(i),now),"startup packet within bound");
    check(!push(queue,32,now),"startup count bounded");
    queue.reset();
    check(queue.size()==0 && queue.bytes()==0 && queue.startup(),"reset discards previous stream");
    check(push(queue,40,now,EncodedFrameQueue::MaxBytes),"exact byte bound accepted");
    check(!push(queue,41,now,1),"aggregate bytes bounded");
    queue.pop(packet);
    check(packet.pts==40 && queue.bytes()==0,"pop releases payload accounting");
    check(!push(queue,42,now,EncodedFrameQueue::MaxBytes+1),"single oversized packet rejected");
    queue.reset();
    check(push(queue,0,now),"new stream accepted after reset");
    check(!queue.expired(now+500ms) && queue.expired(now+501ms),"startup age bounded");
    check(!push(queue,1,now+501ms),"expired reference chain cannot accept more frames");
    queue.decoded();
    check(queue.expired(now+251ms),"steady-state age bounded");
}
void wrap_and_restart() {
    EncodedFrameQueue queue;EncodedPacket packet;
    const auto now=EncodedFrameQueue::Clock::time_point{};
    for(int round=0;round<100;++round) {
        queue.reset();
        for(int i=0;i<24;++i) check(push(queue,i,now),"fill startup ring");
        for(int i=0;i<16;++i) {queue.pop(packet);check(packet.pts==i,"initial FIFO");}
        for(int i=24;i<40;++i) check(push(queue,i,now),"wrap startup ring");
        queue.decoded();
        for(int i=16;i<40;++i) {queue.pop(packet);check(packet.pts==i,"FIFO across wrap and compaction");}
        for(int i=40;i<100;++i) {
            check(push(queue,i,now),"reuse streaming slots");queue.pop(packet);
            check(packet.pts==i && queue.bytes()==0,"streaming FIFO and byte accounting");
        }
    }
}
}
int main() {
    try {
        delayed_decoder_and_drain();bounds_and_reset();wrap_and_restart();
        std::cout<<"PASS: stalled decoder startup, bounded streaming burst, ordered drain, count/byte/age bounds, reset and 100 ring-wrap cycles\n";
    } catch(const std::exception& error) {std::cerr<<error.what()<<'\n';return 1;}
}
