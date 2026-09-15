# WDA trên Windows — thiết lập tùy chọn

[English](WDA_SETUP.en.md) · [Dùng iMirror/Bluetooth](PORTABLE.vi.md)

## Hiểu đúng trước khi bắt đầu

Bluetooth Mouse không cần các bước này. WDA dùng app runner đã ký trên iPhone và
XCTest để gửi tap/swipe. WDA vẫn có độ trễ và vuốt bắt đầu sau khi thả chuột.
Dùng **Wireless cho video + USB cho WDA**; QuickTime USB mirror đã làm gián đoạn
tunnel WDA trên máy được thử. Không mở đồng thời helper/tunnel WDA cũ.

Gói Windows có `WDA\ios.exe`, `wda-forwarder.exe` và script đăng ký. **Không có**
Sideloadly, runner đã ký, Apple developer image, mật khẩu hoặc dữ liệu pairing.
Windows có thể dùng runner dựng sẵn tương thích; muốn tự build/sửa runner cần
Mac/Xcode. Chưa có quy trình WDA một-click được xác nhận trên một máy Windows sạch.

## 1. Chuẩn bị iPhone và Sideloadly

1. Cắm USB, mở khóa, Trust máy tính. Xác nhận Windows nhận thiết bị trước khi ký.
2. Tải Sideloadly Windows x64 trực tiếp từ [sideloadly.io](https://sideloadly.io/).
   Không tải từ bản mirror không rõ nguồn. iMirror chưa có giấy phép đóng gói lại
   Sideloadly nên chỉ cung cấp liên kết, không nhúng installer của họ.
3. Xem [FAQ chính thức](https://sideloadly.io/faq) về iTunes/iCloud cho Windows.
   Các yêu cầu này của Sideloadly khác việc Apple Devices nhận USB. Không thay
   driver đang dùng tốt để thử ngẫu nhiên; xử lý đúng thông báo thiếu thành phần.
4. Lấy bản runner cho **iPhone thật / iphoneos**, không dùng iphonesimulator,
   từ [Appium WebDriverAgent v16.12.8](https://github.com/appium/WebDriverAgent/releases/tag/v16.12.8).
   Đây là version dùng trong thiết lập đã thử; không bảo đảm mọi iOS đều tương thích.

Nếu tải ZIP chứa `WebDriverAgentRunner-Runner.app`, Sideloadly cần IPA có cấu trúc:

```text
Payload/
  WebDriverAgentRunner-Runner.app/
    Info.plist
    ... toàn bộ file/framework/plugin của runner ...
```

Tạo thư mục Payload, giải nén nguyên app vào đó, nén **Payload** thành ZIP rồi đổi
đuôi `.zip` thành `.ipa` (bật View → File name extensions trong Explorer). Không
nén thêm thư mục cha, không bỏ plugin XCTest, không đổi tên ZIP thành IPA nếu
bên trong chưa có Payload. `Invalid file` thường cần kiểm tra lại archive/cấu trúc.
Không dùng IPA đã ký bằng tài khoản của người khác từ một nguồn không xác minh.

## 2. Ký và tin cậy runner

Mở Sideloadly, chọn đúng iPhone và IPA. Dùng Apple Account của chính bạn để ký;
tự nhập mật khẩu/2FA trong Sideloadly, không gửi cho tác giả iMirror. Chờ Done.
Trên iPhone vào **Cài đặt → Cài đặt chung → VPN & Quản lý thiết bị**, chọn ứng dụng
nhà phát triển tương ứng và Tin cậy/Xác minh nếu được yêu cầu.

Theo Sideloadly, chữ ký tài khoản miễn phí thường có hiệu lực 7 ngày; phải gia hạn
bằng tài khoản/bundle phù hợp. iMirror không gia hạn chữ ký. Done hoặc có icon
chưa chứng minh runner/XCTest chạy; nếu ký lỗi nested plugin, cần sửa quy trình ký
của runner, không phải tăng số lần Connect.

## 3. Bật Developer Mode trên iPhone

Vào **Cài đặt → Quyền riêng tư & Bảo mật → Chế độ nhà phát triển**. Bật, đồng ý
khởi động lại. Sau khi máy khởi động, mở khóa, xác nhận Bật và nhập passcode khi
được hỏi. Đây là thao tác trên điện thoại, iMirror không tự bật giúp bạn.

Nếu không có mục này, việc chỉ cắm cáp chưa đủ: hoàn tất cài app development-signed
hoặc developer pairing. Apple mô tả điều kiện xuất hiện và luồng xác nhận trong
[Developer Mode](https://developer.apple.com/documentation/xcode/enabling-developer-mode-on-a-device)
và [WWDC: Get to know Developer Mode](https://developer.apple.com/videos/play/wwdc2022/110344/).
Không dùng profile/web lạ để “mở khóa Developer Mode”. Bluetooth không cần bật chế độ này.

## 4. Chuẩn bị developer image hợp lệ

WDA trên iOS mới cần developer support image tương thích. Lấy image bạn có quyền
sử dụng từ [Apple Developer Downloads](https://developer.apple.com/download/all/)
hoặc môi trường Xcode tương ứng. Với iOS 17+, go-ios cần thư mục **Restore** có
`BuildManifest.plist` và các file image/trustcache đầy đủ. Không chọn file `.dmg`
thay cho thư mục Restore. Không suy ra tương thích chỉ từ tên file hay số build.

Đặt thư mục này ở nơi riêng, ổn định trên máy, ngoài thư mục app sẽ xóa khi nâng
cấp. ZIP iMirror không cung cấp hoặc tự tải Apple image. Nếu bạn chưa có image
tương thích/extract được Restore, bước WDA còn **chưa hoàn thành**; vẫn dùng
Bluetooth Mouse được. Có thể cần người có môi trường Apple/Xcode chuẩn bị image.

## 5. Đăng ký đúng thiết bị trên máy Windows của bạn

Đóng mọi iMirror trước. Trong thư mục app đã giải nén, mở PowerShell (Windows có
sẵn; không cài developer tools). Chạy hai lệnh đọc thông tin:

```powershell
.\WDA\ios.exe list
.\WDA\ios.exe apps --list --udid=YOUR_DEVICE_ID
```

Thay `YOUR_DEVICE_ID` bằng identifier vừa liệt kê. Tìm bundle identifier của
**runner đã cài/ký**; Sideloadly có thể đổi bundle nên không lấy ID ví dụ trong
bài hướng dẫn. Không đăng kết quả đầy đủ lên GitHub.

Chạy file **Configure-WDA.cmd** trong thư mục app. Nhập device ID, runner bundle
ID và đường dẫn Restore khi được hỏi. Script chỉ lưu đăng ký riêng vào
`%LOCALAPPDATA%\iMirror\wda\setup.json`; không xin mật khẩu, không ký/cài app,
không sao chép image vào gói và không chỉnh driver. PowerShell mặc định của Windows
đủ dùng. Nếu đã có pairing folder của chính thiết bị này từ lần thiết lập trước,
có thể dùng script nâng cao:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\register-wda-runtime.ps1 -DeviceId "YOUR_DEVICE_ID" -RunnerBundleId "YOUR.RUNNER.BUNDLE" -DeveloperImagePath "C:\YOUR_IMAGE\Restore" -PairingSource "C:\YOUR_PRIVATE_PAIRING"
```

Không nhập nguyên các chữ YOUR. PairingSource là tùy chọn, không dùng folder của
người khác. Không gửi setup.json, pairing, `.p12` hoặc provisioning profile qua chat.

## 6. Mở và xác nhận điều khiển thật

1. Giữ USB cắm, iPhone mở khóa. Mở đúng một iMirror.
2. **Settings → Advanced → Enable advanced features**. Trong **Control**, chọn
   **Advanced automation (WDA)** và Enable Control.
3. App kiểm tra image, gắn cache khi thiếu, khởi động tunnel/runner. Chờ
   **Advanced control connected**. Mount có thể cần kết nối dịch vụ cá nhân hóa
   của Apple; chưa Ready thì không click thử liên tục.
4. Chọn **Connection → Wireless**, Connect, chọn iMirror trong Phản chiếu màn hình.
5. Mở Máy tính về 0. Click số 1 đúng một lần trên hình. Chỉ coi PASS nếu iPhone
   thật hiện 1. Thử vuốt danh sách bằng giữ chuột, kéo rồi thả; chưa có realtime drag.

Sau restart iPhone, mở khóa và mở lại iMirror. Bản recovery đã được thử tự mount
lại image và nhận click trên một thiết bị. Không bảo đảm tự sửa được chứng chỉ
hết hạn, mất Trust, Developer Mode tắt hoặc image không tương thích sau update iOS.

## Khi lỗi

- **Not Ready:** Settings → Advanced → Diagnostics, xem runtime reason.
- **Image missing:** đăng ký Restore; không xóa cache đã đăng ký khi dọn thư mục app.
- **Signing/trust:** gia hạn đúng runner, tin cậy, kiểm tra bundle identifier.
- **Phone locked / unavailable:** mở khóa bằng passcode, kiểm tra USB/Trust.
- **Port in use / agent not running:** đóng helper thủ công và app khác dùng WDA;
  không kill hàng loạt tiến trình hoặc mở nhiều iMirror để thử.
- **Connected nhưng không bấm:** kiểm tra đang chọn WDA, Control bật, video live.
  Subscriber Bluetooth bằng 0 là bình thường khi đang dùng WDA.

Gửi diagnostics đã che dữ liệu riêng và bước tái hiện. App không thu thập Apple
password. Sideloadly và Apple là bên cung cấp riêng, không thuộc iMirror.
