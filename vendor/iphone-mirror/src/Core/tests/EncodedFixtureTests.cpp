// SPDX-License-Identifier: GPL-3.0-only
// Recorded MIT QVH fixture validation, not a connected-iPhone test.
#include "iPhoneMirror/CoreApi.h"
#include "Protocol/QuickTimePacket.h"
#include "Media/CoreMedia.h"
#include "Media/H264.h"
#include <fstream>
#include <iostream>
#include <iterator>
#include <thread>
#include <chrono>
IM_API std::int32_t IM_CALL im_encoded_session_create(const iPhoneMirror::CaptureOptions*,iPhoneMirror::SessionHandle*);
IM_API std::int32_t IM_CALL im_encoded_session_submit(iPhoneMirror::SessionHandle,const std::uint8_t*,std::uint32_t,
    const std::uint8_t*,std::uint32_t,const std::uint8_t*,std::uint32_t,std::uint32_t,std::uint32_t,std::int64_t,std::int32_t,std::int32_t);
struct Lease {
    iPhoneMirror::SessionHandle handle{};
    ~Lease(){if(handle)im_session_destroy(handle);im_shutdown();}
};
int main() {
    try {
        std::ifstream file("fixtures/quicktime_video_hack/asyn-feed",std::ios::binary);
        if(!file){std::cerr<<"Fixture missing\n";return 1;}
        const std::vector<std::uint8_t> bytes{std::istreambuf_iterator<char>(file),{}};
        iPhoneMirror::quicktime::StreamDecoder parser;
        const auto packets=parser.push(bytes);
        if(packets.size()!=1)return 2;
        const auto envelope=iPhoneMirror::coremedia::parse_sample_envelope(packets[0].payload);
        const auto sample=iPhoneMirror::coremedia::parse_sample_buffer(envelope.serialized_sample_buffer);
        if(!sample.format||sample.format->sequence_parameter_sets.empty()||sample.format->picture_parameter_sets.empty())return 3;
        const auto& format=*sample.format;
        if(!iPhoneMirror::h264::is_keyframe_avcc(sample.sample_data,format.nalu_length_size))return 4;
        if(im_initialize()!=0)return 5;
        Lease lease;
        iPhoneMirror::CaptureOptions options{};
        options.struct_size=sizeof(options);options.api_version=iPhoneMirror::ApiVersion;options.audio_volume=0;
        if(im_encoded_session_create(&options,&lease.handle)!=0)return 6;
        const auto& sps=format.sequence_parameter_sets[0];
        const auto& pps=format.picture_parameter_sets[0];
        for(int i=0;i<20;++i) {
            const auto result=im_encoded_session_submit(lease.handle,sample.sample_data.data(),
                static_cast<std::uint32_t>(sample.sample_data.size()),sps.data(),static_cast<std::uint32_t>(sps.size()),
                pps.data(),static_cast<std::uint32_t>(pps.size()),format.width,format.height,
                (i+1)*333333LL,1,i==0);
            if(result!=0){std::wcerr<<im_last_error()<<L'\n';return 7;}
            std::this_thread::sleep_for(std::chrono::milliseconds(50));
        }
        for(int i=0;i<100;++i) {
            iPhoneMirror::CaptureStatus status{};
            status.struct_size=sizeof(status);status.api_version=iPhoneMirror::ApiVersion;
            if(im_session_get_status(lease.handle,&status)!=0)return 8;
            std::int64_t timestamp{};
            im_session_get_latest_video_timestamp(lease.handle,&timestamp);
            if(timestamp>0&&status.state==iPhoneMirror::CaptureState::Streaming) {
                iPhoneMirror::VideoOutputStatus output{};
                output.struct_size=sizeof(output);output.api_version=iPhoneMirror::ApiVersion;
                if(im_session_get_video_output_status(lease.handle,nullptr,&output)!=0)return 9;
                std::cout<<"Recorded fixture decoded: "<<status.width<<"x"<<status.height
                    <<" decoder_mode="<<static_cast<unsigned>(output.decoder_runtime_mode)
                    <<" last_decode_ms="<<status.latency_ms<<"\n";
                return status.width==format.width&&status.height==format.height?0:10;
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(50));
        }
        std::cerr<<"Recorded fixture did not decode\n";return 11;
    } catch(const std::exception& error){std::cerr<<error.what()<<'\n';return 12;}
}
