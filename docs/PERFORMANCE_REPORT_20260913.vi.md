# iMirror — báo cáo thử nghiệm hiệu năng 13/09/2026

Đã tạo bản Windows experimental, chưa phải bản stable/public release. USB M3 có
cải thiện rõ ở đuôi frame interval khi cuộn; chưa giải quyết mọi jitter. WDA T10
là lựa chọn thử riêng, không thay mặc định 50 ms. Realtime swipe chưa được
implement. Không merge main, không đổi checkpoint, không ghi đè EXE stable.

## Bản thử và cách mở

EXE mới:
`D:\airplay-iphone\dist\imirror-performance-experimental-20260913\iMirror.exe`

SHA256: `0FA2692831B65FB09109287325826821BD1E75AE286B95BCB8DD22F30D4BFDE4`.
Kích thước: 3.453.440 byte. Version 0.1.0. **UNSIGNED DEVELOPMENT EXPERIMENT**.

Đóng iMirror đang mở rồi dùng đúng một launcher trong cùng thư mục:

- `Start-USB-M3.cmd`: bật GPU handoff/cache và blocking Present cho USB; VSync
  giữ nguyên. Chọn USB + Bluetooth Mouse; WDA USB và chế độ capture USB có thể
  xung đột trên thiết bị này.
- `Start-WDA-T10.cmd`: bật WDA tap 10 ms; dùng Wireless + USB cho WDA. Giữ nguyên
  đường video Wireless. Chưa có realtime swipe.
- `Start-Baseline.cmd`, hoặc mở EXE bình thường: CPU readback/Present cũ và WDA
  contact 50 ms. Các tối ưu đều mặc định OFF.

Launcher không gửi tap tự động và không mở cửa sổ diagnostics. Runtime riêng
được giữ bên EXE; không dùng DLL từ PATH của môi trường dev. Bản mới đã stage,
**chưa được mở để thay phiên đang chạy**, chờ người dùng thử thủ công.

## 1. Nguyên nhân jitter của mirror

Có nhiều nguồn, không đủ dữ liệu để gán mọi spike cho renderer:

- Source PTS tự có khoảng trống: test C gốc P99 **33,282 ms**, dù median gần
  16,67 ms. Arrival trên PC P99 **38,552 ms**; display P99 **50,072 ms**.
- Đường cũ dùng hardware decode nhưng đọc GPU về CPU rồi upload lại. Profile
  xác nhận `Map` chiếm trung bình khoảng **6,8–7,0 ms** trong các lượt profile
  A/B; memcpy khoảng **0,9 ms**. GPU copy submission chỉ khoảng 0,004 ms, không
  phải thời gian GPU hoàn tất.
- Có thêm jitter ở decode publication và Windows display scheduling. Test C
  gốc có **411/133/58** interval >25/>40/>50 ms. M3 C còn **456/27/10**: đuôi
  lớn giảm, nhưng số interval >25 ms không giảm.

Trong 133 spike >40 ms của C gốc, ngưỡng lần đầu bị vượt tại decoded publication
96 lần, host observation 25, source PTS 4, Windows display 5; 3 khoảng có frame
nguồn không được hiển thị ở giữa. Đây là tương quan cùng cặp frame, không phải
chứng minh độc quyền nguyên nhân. Burst quan sát trên host chưa tách được hoàn
toàn iPhone, USB driver và scheduler của PC. Không đổi timestamp/queue để làm số đẹp.

## 2. USB trước và sau

Cùng H.264 **1180×2556**, hardware Media Foundation, D3D11, VSync ON, viewport
564×1222 ở DPI 144. Mỗi A/B/C ít nhất 60 giây. Display FPS dùng **PTS duy nhất
được ETW xác nhận hiển thị**, không dùng số lần gọi Present.

| Lượt | Source FPS | Decode FPS | Display FPS | Avg interval ms | P50 | P95 | P99 | Max | 1% low FPS |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Gốc A đứng yên | 41,016 | 41,016 | 40,916 | 24,406 | 16,706 | 53,097 | 83,444 | 108,918 | 10,381 |
| M3 A đứng yên | 40,858 | 40,858 | 40,044 | 24,961 | 16,708 | 52,665 | 75,124 | 108,535 | 10,688 |
| Gốc B cuộn | 57,161 | 57,161 | 57,044 | 17,531 | 16,691 | 33,379 | 50,083 | 66,788 | 17,765 |
| M3 B cuộn | 58,113 | 58,113 | 58,030 | 17,232 | 16,693 | 25,072 | 41,726 | 58,421 | 21,110 |
| Gốc C cuộn nhanh | 59,013 | 58,996 | 58,896 | 16,977 | 16,688 | 25,212 | 50,072 | 75,029 | 18,875 |
| M3 C cuộn nhanh | 58,460 | 58,460 | 58,477 | 17,103 | 16,692 | 26,028 | 34,677 | 67,387 | 22,661 |

Source khác nhẹ giữa các lượt cuộn tay; không phải deterministic replay.
Display cao hơn source một chút ở M3 C là chênh một frame tại biên cửa sổ đo.
Không tạo/interpolate frame mới. 1% low = 1000 / trung bình 1% interval chậm nhất.

C là kết quả tốt hơn ở P99/1% low, **không phải đều 60 FPS**; P95 còn tăng nhẹ.
M3 A vẫn có 45 decoded outputs không được Present trước tail (~1,8%). B có 2,
C có 0. Không ghi nhận encoded queue drop trong các lượt USB này. Chưa được dùng
kết quả này để tuyên bố đạt mọi release gate.

| C cuộn nhanh | Gốc | M2 CPU + blocking Present | M3 GPU + cache + blocking Present |
|---|---:|---:|---:|
| Decode avg / P95 / P99 ms | 8,108 / 11,121 / 16,500 | 8,507 / 11,115 / 14,867 | 0,612 / 0,828 / 1,804 |
| Render avg / P95 / P99 ms | 0,660 / 0,900 / 3,501 | 0,648 / 0,946 / 2,241 | 0,813 / 3,680 / 7,117 |
| Receive→display avg / P95 ms | 24,044 / 34,596 | 23,278 / 29,092 | 21,306 / 26,594 |
| CPU trung bình (% cả 12 logical CPU) | 1,752 | 1,600 | 0,738 |
| WAS_STILL_DRAWING | 725 (16,97% attempts) | 0 | 0 |

Render wall time có cả thời gian chờ Present; không tương đương GPU execution
time. Receive→display không gồm capture/encode và thời gian trước khi PC nhận.

## 3. GPU handoff / Present

Đã thử **minimal-copy**, không phải zero-copy tuyệt đối: MF DXGI texture → một
GPU copy sang shared texture → renderer. M3 không có per-frame Map/memcpy CPU
trong các sample đã đo. Cache import/view có 16 slots, không giữ strong reference
làm cạn producer pool. Kiểm tra adapter và có fallback CPU khi consumer lỗi.
Fault injection adapter mismatch/device loss chưa được thử trên hardware.

M1 riêng làm giảm CPU nhưng lặp A có kiểm soát mất 177 outputs, P99 tăng
58,451→66,706 ms: **REJECT standalone**. M1b cache + DO_NOT_WAIT tạo khoảng
3000 attempts/s ở smoke: dừng trước benchmark, không giữ riêng. M3 dùng cache
cùng blocking Present; không busy-spin, không tắt VSync, không đổi queue/quality.
WAS_STILL_DRAWING về 0 là hệ quả đổi flags, tự nó không chứng minh latency tốt hơn.

## 4. WDA tap và vị trí thời gian chậm

Lượt 008: đủ **100 tap/mức**, kiểm chứng Máy tính thật đổi 0→1 sau mỗi tap.
400/400 PASS, 0 thiếu, 0 sai, 0 không xác minh được. Đo bằng helper Rust dùng
cùng persistent client/session; không có geometry query thừa trong tap.

| Contact ms | PASS | HTTP avg ms | P50 | P95 | P99 | Max |
|---:|---:|---:|---:|---:|---:|---:|
| 50 | 100/100 | 419,438 | 394,516 | 535,136 | 620,604 | 662,736 |
| 30 | 100/100 | 423,509 | 394,293 | 540,420 | 610,341 | 791,037 |
| 20 | 100/100 | 414,534 | 393,914 | 538,317 | 613,542 | 684,888 |
| 10 | 100/100 | 415,016 | 392,042 | 533,208 | 545,200 | 547,200 |

Median HTTP chỉ giảm 2,473 ms ở 10 ms; **chưa đạt <300 ms HTTP**. P99 tốt hơn
trong lượt này, không phải guarantee cho những lần chạy khác. Lượt này không
dùng ROI/ETW vì Wireless đã vào recovery, nên không phải benchmark mirror khỏe.

Lượt 007 có video khỏe trong phần lớn phép đo: 163 tap thật PASS, **162 mẫu ROI
đối chiếu ETW**, sau đó Wireless mất live. Thời điểm bắt đầu là generated dispatch,
không phải physical mouse switch; ROI là kết quả Máy tính, không phải key highlight.

| Contact | Số mẫu visual | Dispatch→kết quả hiển thị avg / P50 / P95 / P99 ms |
|---:|---:|---|
| 50 | 41 | 323,167 / 313,117 / 459,318 / 478,845 |
| 30 | 41 | 304,776 / 305,449 / 420,237 / 429,585 |
| 20 | 40 | 302,477 / 296,210 / 437,194 / 564,642 |
| 10 | 40 | 294,887 / 294,149 / 421,465 / 451,349 |

Không ghép 007/008 thành một phân bố end-to-end. Visual chưa đủ 100 mẫu/mức;
không gán mẫu bị mất bằng thời gian HTTP. T10 được giữ **opt-in để thử thủ công**,
mặc định vẫn 50 ms, native fallback và session/geometry/replay safeguards giữ nguyên.

Snapshot người dùng gửi có một tap UI: mouse-up→enqueue 0,0142 ms, queue 0,0154 ms,
dispatch→HTTP 0,0134 ms, HTTP 718,171 ms. Swipe tương ứng 0,0706 / 0,0150 /
0,0162 / 979,884 ms. Đây chỉ là một mẫu mỗi loại; không đủ P95/P99. Mouse-down
bao gồm thời gian người dùng giữ nút (khoảng 74/91 ms), phải tách khỏi queue.

Rust preparation trong lượt 008 chỉ khoảng 0,012–0,013 ms. Phần chờ headers
chiếm gần toàn bộ HTTP. **HTTP/tunnel, routing WDA và XCTest chưa được tách
riêng**, không lấy `/status` rồi trừ để giả ra XCTest. Kết quả hình thường xuất
hiện trước HTTP completion, nên giảm completion tail chưa chắc giảm first response.

## 5. Swipe và custom runner

Swipe cũ và mới vẫn: mouse-down lưu start → mouse-up phân loại → gửi gesture
hoàn chỉnh. **Không gửi touchMove trong lúc kéo. Realtime swipe NOT IMPLEMENTED.**
Không làm chuỗi mini-swipes giả realtime. Fallback swipe giữ nguyên.

Pinned WDA có vòng chờ completion 100 ms và header có persistent-state fields;
chưa chứng minh được continuous touch xuyên request. Đã chuẩn bị
`experiments/wda-timing/runner-timing.patch` để đo lookup app, dựng path, submit,
callback và completion wait. Patch áp dụng đúng pinned blobs; build script qua
kiểm tra cú pháp. **Chưa compile Objective-C, chưa sign/cài/test trên iPhone.**
Không đổi transport, spinner hay inject touch state trong patch đo này.

Không có Mac/Xcode tại chỗ. Có thể build qua GitHub macOS CI, nhưng chưa chạy
job/đẩy workflow; repo private có thể phát sinh phí runner. Bản runner mới cần
người dùng ký/cài và giữ phương án quay lại runner hiện tại.

## 6. Reliability và giới hạn phép đo

- Lượt 004 dừng sau 214 tap: 213 PASS, 1 chưa xác minh sau HTTP interruption.
  Windows System log có Modern Standby/Idle Timeout; loại khỏi acceptance.
- Bộ đo mới giữ PC/display thức trên thread của helper và trả lại khi thoát;
  không thay power plan hoặc startup của app thường.
- Lượt 007 không có Modern Standby nhưng Wireless bị đầy 16 encoded packets,
  chuyển chờ keyframe. Tap cuối vẫn đổi kết quả trên phone nhưng mất frame ROI.
  Đây là vấn đề video/recovery, chưa được sửa bằng việc nới queue/hạ chất lượng.
- USB M3 scoped candidate trước đó được người dùng xác nhận màu/hình live,
  Rotate, fullscreen, resize, minimize/restore, button reconnect đều bình thường.
  Không dùng xác nhận đó làm PASS cho EXE mới tích hợp T10.
- BLE/Raw Input/keyboard không đổi thuật toán; smoke trên EXE cuối, rút/cắm cáp,
  forced GPU fallback, 30 phút memory/handle leak và clean Windows còn chưa xác minh.

## 7. RAM và tài nguyên

M2 private bytes trung bình A/B/C: **201,668 / 202,882 / 204,197 MiB**.
M3: **186,737 / 188,013 / 188,810 MiB**.
Working set M2: 169,862 / 171,073 / 31,572 MiB; M3: 166,417 / 121,267 / 123,036 MiB.
M2 C bị Windows trim working set nên không dùng 31,572→123,036 để kết luận tăng
allocation. Run gốc không có RAM measurement; không báo 0. Chưa đo riêng GPU
memory/processing, handle growth, startup hay 30 phút leak freedom ở phase này.

## 8. Quyết định thí nghiệm

| Thí nghiệm | Thay đổi | Quyết định và trade-off |
|---|---|---|
| P0 | QPC bounded + ETW, đo copy/stage | KEEP instrumentation; chỉ bật recorder khi đo |
| M1 | GPU share, Present cũ | REJECT standalone: drop/pacing xấu hơn dù CPU giảm |
| M1b | Cache import, Present cũ | REJECT standalone: rapid retry; chưa có A/B/C |
| M2 | CPU + blocking Present | Giữ lựa chọn so sánh experimental; không chứng minh universal win |
| M3 | GPU/cache + blocking Present | KEEP opt-in; C P99/CPU tốt hơn, A vẫn drop, chờ manual |
| T30/T20 | Contact 30/20 ms | Không đưa vào app; chưa có lợi ích đủ rõ hơn T10 |
| T10 | Contact 10 ms | KEEP opt-in; 100 Calculator PASS, HTTP median cải thiện nhỏ, visual còn ít mẫu |
| HTTP transport rewrite | Không thực hiện | Chưa có bằng chứng cần rewrite |
| Runner timing | Patch đo riêng phía WDA | Prepared/uncompiled; chưa là fast path |
| Continuous touch | Chỉ nghiên cứu | NOT IMPLEMENTED, không dùng approximation |

## 9. Build, files và Git

`cargo fmt --check`, strict all-target/all-feature Clippy, `cargo test` (**75 tests**),
`cargo build --release`: **PASS**. WDA all-feature tests: **19 PASS**. Analyzer
ROI/ETW có 4 tests; cần đọc số mẫu/gate, không dùng test phần mềm thay hardware.
PE audit kiểm tra **83 EXE/DLL**: imports tìm được trong private runtime hoặc
Windows System32/API sets; 82 runtime binaries giữ hash của stable. Không phải
clean-machine validation, không kiểm tra mọi dynamic plugin trên máy khác.

Các nhóm file thay đổi:

- App: `benchmark.rs`, `main.rs`, `usb_pacing.rs`, `visual_lab.rs`, `ui.rs`
  (chỉ hooks đo), `control.rs`, `wda_input_trace.rs`.
- WDA: `Cargo.toml`, `lib.rs`, `cache_tests.rs`, `timing.rs`, `probe.rs`,
  `probe_visual.rs`, example `wda-tap-probe.rs`.
- Native: `CaptureSession.cpp` (hooks/USB opt-in), `CoreApi.cpp`,
  `MediaFoundationDecoder.cpp/.h`, `SourceFrameCounter.h`,
  `D3D11PreviewRenderer.cpp/.h`, `Diagnostics/FramePacing.h`, `VisualProbe.h`.
- Offline scripts: `analyze-usb-pacing.py`, `analyze-usb-spikes.py`,
  `analyze-wda-visual.py`; experiment WDA timing và các báo cáo/engineering log.

Branch: `experiment/performance-20260913`. Base/checkpoint commit:
`bfa4784a88adf793052b44ad758fa9d7ec7994ba`. Stable EXE SHA256 vẫn:
`C577805DB7CD72AC01A52AB2D7E83EDD9F450F9C342941C1957FBE32BAF54DB0`.
Code đã lưu ở commit local `c17b5b0` (USB/native) và `eee5b9e` (WDA timing/T10).
Không push/merge main, không tag/release mới. Xem manifest của EXE để đối chiếu
hash source đã build. Các raw CSV/JSON nằm trong `work/usb-pacing-20260913`
và `work/performance-20260913`, không đưa generated output vào Git.

Trạng thái hiện tại: dừng thay đổi để người dùng thử bản experimental. Chưa đạt
mọi mục tiêu latency/realtime/reliability; không đánh dấu toàn bộ project hoàn tất.
