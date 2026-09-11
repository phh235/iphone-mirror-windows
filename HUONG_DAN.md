# iMirror — hướng dẫn nhanh

Chạy iMirror.exe trong thư mục đã giải nén đầy đủ, hoặc cài bằng Setup.exe/MSI.
Giữ các DLL, chương trình hỗ trợ USB và thư mục AirPlay bên cạnh ứng dụng.
Máy người dùng không cần Rust, Python, Node.js, CMake hay FFmpeg CLI.

## USB

1. Kết nối iPhone bằng cáp, mở khóa và chọn Tin cậy máy tính nếu được hỏi.
2. Chọn iPhone trong ứng dụng rồi bấm Connect.
3. Dùng Rotate, 1:1/Fit và Fullscreen theo nhu cầu.
4. Escape nhả điều khiển; Ctrl+Shift+F đổi chế độ toàn màn hình khi vùng video có tiêu điểm.

Hình ảnh USB thật và xoay màn hình đã được người dùng xác nhận trên thiết bị thử.
Khả năng tương thích còn phụ thuộc phiên bản iOS và driver Windows. Ứng dụng
không tự cài driver hoặc thay thế driver Apple.

## Điều khiển và âm thanh

BLE control dùng kết nối Bluetooth riêng; cần ghép iPhone với PC, bật
AssistiveTouch khi cần, rồi chọn kết nối BLE trong ứng dụng. Đây là con trỏ
tương đối. WDA control là tùy chọn điều khiển tọa độ chính xác, cần runner đã
được ký và tunnel cục bộ tại 127.0.0.1:8100. Những thao tác này còn cần thử thật.

Mute/Unmute điều khiển âm thanh. Settings lưu tên bộ nhận AirPlay, chất lượng,
VSync và tự kết nối lại. Thay tên/chất lượng AirPlay sẽ khởi động lại bộ nhận.

## AirPlay

Đặt iPhone và PC cùng mạng nội bộ. Bấm AirPlay, rồi mở Phản chiếu màn hình
trên iPhone và chọn iMirror. Nếu Windows hỏi quyền mạng, tự chọn đúng mạng
riêng bạn muốn dùng. Luồng AirPlay thật chưa hoàn tất kiểm chứng.

## Kết quả và giới hạn

FPS hiển thị là FPS nguồn đo được, có thể thay đổi theo cảnh và thiết bị.
Thời gian decode không phải độ trễ từ màn hình iPhone đến màn hình PC.
Xem docs/VALIDATION.md để biết các phép thử đã chạy và các mục còn thiếu.

Gỡ cài đặt qua mục Ứng dụng đã cài đặt của Windows. Bộ cài chưa có chữ ký số.
Bản hiện tại là bản phục vụ kiểm chứng, chưa được chứng nhận hoàn tất các
tiêu chí phát hành sản xuất. Sau bài thử dài, bạn có thể đặt lại Tự động khóa
trên iPhone về thời gian trước đó.
