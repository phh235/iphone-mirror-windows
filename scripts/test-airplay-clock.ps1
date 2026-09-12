#Requires -Version 7.0
# Regression test of the pinned UxPlay callback with a simulated renderer clock.
# Run scripts/build-airplay-helper.ps1 first to obtain the pinned local compiler.
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
$root=(Resolve-Path -LiteralPath (Split-Path $PSScriptRoot -Parent)).Path
$source=Get-Content -LiteralPath (Join-Path $root 'vendor/uxplay/uxplay.cpp') -Raw
$begin=$source.IndexOf('extern "C" void video_process (')
if($begin -lt 0){throw 'Cannot locate the pinned video callback.'}
$end=$source.IndexOf("`n#ifdef DBUS",$begin)
if($end -lt 0){throw 'Cannot locate the end of the pinned video callback.'}
$callback=$source.Substring($begin,$end-$begin)
$preamble=@'
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <vector>
struct raop_ntp_t {};
struct video_decode_struct {
  unsigned char *data; int data_len, nal_count;
  uint64_t ntp_time_remote, ntp_time_local;
};
static bool dump_video=false, mux_to_file=false, use_video=true;
static uint64_t remote_clock_offset=0;
static constexpr uint64_t SECOND_IN_NSECS=1000000000ULL;
static constexpr uint64_t pipeline_base=70003516000ULL;
static std::vector<uint64_t> delivered;
void dump_video_to_file(unsigned char*,int) {}
void mux_renderer_push_video(unsigned char*,int,uint64_t) {}
uint64_t get_local_time() { return 70000000000ULL; }
#define LOGI(...) ((void)0)
// Test sink emulates the documented renderer clock-underflow retry contract.
uint64_t video_renderer_render_buffer(unsigned char*,int*,int*,uint64_t *pts) {
  if (*pts < pipeline_base) return pipeline_base-*pts;
  delivered.push_back(*pts-pipeline_base);
  return 0;
}
'@
$test=@'
int main() {
  unsigned char bytes[4]{};
  video_decode_struct first{bytes,4,1,50000000000ULL,70000000000ULL};
  video_process(nullptr,nullptr,&first);
  video_decode_struct next{bytes,4,1,50016640000ULL,70016640000ULL};
  video_process(nullptr,nullptr,&next);
  if (delivered.size()!=2 || delivered[0]!=0 || delivered[1]!=16640000ULL) {
    std::fprintf(stderr,"FAIL: first PTS=%llu, second PTS=%llu; expected 0 and 16640000 ns\n",
      delivered.empty()?0:(unsigned long long)delivered[0],
      delivered.size()<2?0:(unsigned long long)delivered[1]);
    return 1;
  }
  // Starting after the pipeline base must preserve an ordinary source delta.
  delivered.clear(); remote_clock_offset=0;
  video_decode_struct normal{bytes,4,1,50000000000ULL,71000000000ULL};
  video_process(nullptr,nullptr,&normal);
  video_decode_struct normal_next{bytes,4,1,50016640000ULL,71016640000ULL};
  video_process(nullptr,nullptr,&normal_next);
  if(delivered.size()!=2 || delivered[1]-delivered[0]!=16640000ULL) return 2;
  std::puts("PASS: actual UxPlay callback preserves source deltas with and without clock-underflow retry. Simulated sink only; no physical video tested.");
}
'@
$testDirectory=Join-Path $root 'work/airplay-clock-test'
New-Item -ItemType Directory -Path $testDirectory -Force | Out-Null
$testFile=Join-Path $testDirectory 'video-clock-test.cpp'
[IO.File]::WriteAllText($testFile,($preamble+"`n"+$callback+"`n"+$test),[Text.UTF8Encoding]::new($false))
$bin=Join-Path $root 'work/airplay-helper-build/sdk/ucrt64/bin'
$env:PATH=$bin+';'+(Join-Path $env:SystemRoot 'System32')
$testExe=Join-Path $testDirectory 'video-clock-test.exe'
& (Join-Path $bin 'g++.exe') -std=c++17 -O2 -Wall -Wextra -Werror -Wno-unused-parameter $testFile -o $testExe
if($LASTEXITCODE -ne 0){throw 'Clock test build failed'}
& $testExe
exit $LASTEXITCODE
