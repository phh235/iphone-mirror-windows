// SPDX-License-Identifier: GPL-3.0-only
#include "Media/SourceFrameCounter.h"
#include <iostream>
int main() {
    using namespace iPhoneMirror;
    media::SourceFrameCounter counter;
    coremedia::SampleBuffer sample;
    sample.sample_count = 1;
    sample.sample_data = {1};
    sample.output_presentation_timestamp = coremedia::CMTime{0, 600, 1, 0};
    for (int i=0;i<60;++i) {
        sample.output_presentation_timestamp->value = i * 10;
        counter.observe(sample);
        counter.observe(sample);
    }
    if (counter.unique()!=60 || counter.duplicates()!=60 || !counter.timing_complete()) return 1;
    sample.output_presentation_timestamp->epoch=1;
    counter.observe(sample);
    if (counter.unique()!=61) return 2;
    sample.output_presentation_timestamp->flags=0;
    counter.observe(sample);
    if (counter.unique()!=61 || counter.timing_complete()) return 3;
    counter.reset();
    sample.sample_count=3;
    sample.timing.resize(3);
    for (int i=0;i<3;++i) sample.timing[i].presentation_timestamp={i*10,600,1,0};
    counter.observe(sample);
    if (counter.unique()!=3) return 4;
    sample.sample_data.clear();
    counter.observe(sample);
    if (counter.unique()!=3) return 5;
    std::cout << "Source sample count, duplicate, epoch and invalid-timestamp tests passed\n";
}
