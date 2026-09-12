# Hướng dẫn sử dụng iMirror

[English guide](docs/USER_GUIDE.md) · [README](README.md)

iMirror phản chiếu màn hình iPhone lên Windows và hỗ trợ điều khiển bằng chuột,
bàn phím qua Bluetooth. USB và chuột tương đối đã được thử trên iPhone thật;
khả năng tương thích nhiều thiết bị, Wireless và bộ cài trên Windows sạch vẫn
cần kiểm chứng. Xem [trạng thái kiểm thử](docs/VALIDATION.md).

## 1. Chuẩn bị và mở ứng dụng

- Windows 11 x64 được ưu tiên; Windows 10 22H2 phụ thuộc API và driver khả dụng.
- Dùng cáp USB có truyền dữ liệu, mở khóa iPhone và chấp nhận **Tin cậy máy tính
  này / Trust This Computer** nếu được hỏi.
- USB cần phần mềm/driver Apple tương thích. Nếu máy chưa có, dùng
  [Apple Devices từ nguồn chính thức](https://support.apple.com/en-us/118290).
  Cài Apple Devices không bảo đảm mọi cấu hình USB sẽ tương thích với iMirror.
- Điều khiển chuột cần Bluetooth trên Windows có hỗ trợ vai trò thiết bị ngoại
  vi BLE. Nếu adapter không hỗ trợ, bạn vẫn có thể dùng chế độ chỉ phản chiếu.

Với bản cài đã được kiểm chứng: cài bằng Setup rồi mở từ Start Menu hoặc shortcut.
Với thư mục thử nghiệm được cung cấp: giải nén đầy đủ và mở `iMirror.exe`, giữ
nguyên các DLL, chương trình hỗ trợ USB, thư mục `AirPlay` và giấy phép đi kèm.
Không lấy riêng EXE ra khỏi thư mục. Máy sử dụng app không cần Rust, Git, Python,
Node.js, CMake hay Visual Studio. Bản hiện tại chưa hoàn tất kiểm chứng bộ cài
trên Windows sạch; đừng nhầm một bản build thành công với bản phát hành hoàn chỉnh.

## 2. Kết nối USB

Đổi ngôn ngữ tại **Settings → General → Language** hoặc
**Cài đặt → Chung → Ngôn ngữ**: chọn **English** hay **Tiếng Việt**. App đổi ngay
và lưu lựa chọn cho lần mở sau. Tên thiết bị, tên API, dữ liệu chẩn đoán và một số
thông báo gốc từ driver/hệ điều hành được giữ nguyên.

1. Cắm cáp và mở khóa iPhone.
2. Mở iMirror. **Automatic** thử kết nối iPhone USB được tìm thấy.
3. Nếu chưa có video, mở **Settings → Connection**, chọn **USB**, chọn iPhone
   trong mục **Device**; dùng **Refresh** nếu cần. Đóng Settings và bấm icon
   **Connect** trên thanh trên cùng.
4. Khi trạng thái thành **Connected**, kiểm tra hình bằng cách thao tác trực tiếp
   trên iPhone. Di chuột lên icon để xem tooltip tên chức năng.

Trong **Fit / Vừa khung**, cửa sổ thường tự thu theo tỷ lệ video khi có format
đầu tiên, khi xoay hoặc kết nối lại. Kéo cạnh cửa sổ vẫn giữ tỷ lệ đó; app tính
riêng toolbar, thanh tiêu đề và DPI. Hình không bị kéo giãn hay cắt để loại bỏ
viền. Fullscreen có thể có dải đen do tỷ lệ màn hình PC khác iPhone; đó là bình
thường. Nếu bạn chủ động chọn **1:1** hoặc **Fill / Lấp đầy**, app giữ lựa chọn ấy.

Không cần bật Control, cài app lên iPhone, đăng nhập Apple ID, bật Developer Mode
hay jailbreak chỉ để dùng đường USB và chuột Bluetooth của iMirror. Không tự
thay driver USB cha của iPhone bằng WinUSB; nếu không kết nối được, xem phần xử
lý lỗi bên dưới.

## 3. Các nút trên thanh công cụ

Từ trái sang phải ở phía bên phải cửa sổ:

| Tooltip | Chức năng |
| --- | --- |
| Connect / Disconnect | Một nút đổi theo trạng thái kết nối |
| Rotate | Xoay cách hiển thị video trên PC |
| Control iPhone | Bật hoặc tắt điều khiển |
| Fullscreen / Exit fullscreen | Vào hoặc thoát toàn màn hình |
| Settings | Mở cài đặt |

Khi cửa sổ hẹp, Connect và Rotate có thể nằm trong menu **…**. Nút Home chỉ có
trong menu khi backend nâng cao đang dùng thực sự hỗ trợ chức năng đó.

## 4. Ghép chuột Bluetooth lần đầu

1. Bật Bluetooth trên Windows.
2. Trong iMirror, bấm **Control** hoặc mở **Settings → Control → Enable Control**.
   Giữ **Input mode: Automatic** hoặc **Bluetooth Mouse**.
3. Chờ hướng dẫn ghép đôi xuất hiện. Nếu app đang báo bật Bluetooth hoặc adapter
   không hỗ trợ, xử lý thông báo đó trước.
4. Trên iPhone, vào **Cài đặt → Trợ năng → Cảm ứng → AssistiveTouch** và bật
   AssistiveTouch. Mở **Thiết bị → Thiết bị Bluetooth**, chọn tên PC đang quảng bá.
   Hoàn tất yêu cầu ghép đôi trên cả hai máy nếu có.
5. Chờ mục **Status** của iMirror báo **Ready**, rồi bấm trong vùng video để thu
   nhận chuột/bàn phím.

[Hướng dẫn AssistiveTouch chính thức của Apple](https://support.apple.com/en-us/111775).
Tên menu có thể khác đôi chút tùy ngôn ngữ và phiên bản iOS. Zoom không phải yêu
cầu của Bluetooth Mouse.

Bluetooth báo **Connected** chưa có nghĩa là iMirror đã sẵn sàng điều khiển.
Nếu app chưa báo Ready, đừng thử các công cụ gửi report trong Diagnostics để
thay cho việc hoàn tất ghép đôi.

## 5. Điều khiển hằng ngày

| Thao tác | Kết quả |
| --- | --- |
| Bấm trong vùng video khi Control sẵn sàng | Thu nhận input cho iPhone |
| Di chuột | Di chuyển con trỏ AssistiveTouch trên iPhone |
| Bấm chuột trái | Kích hoạt tại vị trí con trỏ iPhone |
| Giữ trái và kéo | Kéo hoặc vuốt, tùy ứng dụng iPhone |
| Lăn bánh xe | Cuộn dọc |
| Gõ khi ô nhập trên iPhone được chọn | Gửi phím qua Bluetooth nếu kết nối bàn phím sẵn sàng |
| Ctrl+Alt+Q | Trả điều khiển ngay cho Windows |
| Esc | Nhả input đang được thu nhận |
| Ctrl+Shift+F khi vùng video có focus | Đổi chế độ toàn màn hình |

Đây là **chuột tương đối**: vị trí chuột Windows không phải tọa độ chạm tuyệt đối
trên iPhone. Nhìn con trỏ AssistiveTouch để biết vị trí bấm. Khả năng nhập ký tự,
bộ gõ và phím đặc biệt còn phụ thuộc bố cục bàn phím/iOS; chưa có xác nhận cho mọi
ngôn ngữ. App hiện không phát âm thanh.

Trong **Settings → Control**, chỉnh **Pointer sensitivity** từ 5–200%.
Mặc định cho cấu hình mới là **100%**; giá trị bạn đã lưu được giữ lại. Giảm nếu
con trỏ quá nhạy, tăng nếu di chuyển quá ngắn. Gia tốc chuột của iOS có thể khiến
cảm giác khác Windows; tăng độ nhạy không loại bỏ độ trễ Bluetooth.

## 6. Settings

- **General / Chung:** chọn English hoặc Tiếng Việt.
- **Connection:** Automatic, USB, Wireless; chọn và làm mới danh sách thiết bị.
  **Receiver name** chỉ hiện khi chọn Wireless. Đổi chế độ sẽ ngắt kết nối hiện tại.
- **Control:** Enable Control, Input mode, Pointer sensitivity, phím nhả và trạng thái.
- **Display:** **Fit** giữ toàn bộ hình; **1:1** dùng một pixel nguồn cho một pixel
  hiển thị; **Fill** lấp cửa sổ và có thể cắt mép. **Synchronized display** bật/tắt
  đồng bộ trình bày hình.
- **Advanced:** Diagnostics, Copy Diagnostics và các tùy chọn nâng cao. Cài đặt
  thông thường được lưu tự động; dùng Done để đóng cửa sổ.

## 7. Wireless — còn cần kiểm chứng trên thiết bị thật

1. Cho PC và iPhone vào cùng mạng nội bộ.
2. Trong **Settings → Connection**, chọn **Wireless** và đặt **Receiver name**.
3. Đóng Settings, bấm **Connect**.
4. Mở Trung tâm điều khiển của iPhone → **Phản chiếu màn hình / Screen Mirroring**
   và chọn tên receiver.
5. Nếu Windows hỏi quyền mạng, chỉ cho phép mạng riêng mà bạn định dùng. Không
   cần tắt tường lửa toàn bộ.

USB và Wireless dùng kết nối khác nhau. Ghép Bluetooth chỉ phục vụ điều khiển,
không truyền video. Wireless cần runtime đi kèm đầy đủ; hiện chưa có kết quả
FPS/độ trễ được xác nhận trên iPhone thật cho đường này.

## 8. Xử lý lỗi

| Hiện tượng | Cách kiểm tra |
| --- | --- |
| Không tìm thấy iPhone USB | Mở khóa, kiểm tra cáp dữ liệu/cổng USB, chấp nhận Trust, dùng Refresh và kiểm tra Apple Devices nhận được máy |
| Có tên máy nhưng không có video | Chọn đúng thiết bị, bấm Connect; nếu vẫn lỗi, Copy Diagnostics để báo lỗi, không đổi driver bừa |
| Bluetooth Connected nhưng Control chưa Ready | Chờ trạng thái; kiểm tra AssistiveTouch. Nếu cần ghép lại, Forget PC trên iPhone và Remove iPhone trong Windows, rồi ghép từ AssistiveTouch → Devices |
| Không thấy PC trong danh sách Bluetooth | Giữ Control bật và iMirror mở; kiểm tra Bluetooth Windows, khả năng adapter và thông báo trong Settings |
| Con trỏ quá nhanh/chậm | Chỉnh Pointer sensitivity, đồng thời kiểm tra tracking speed của iPhone |
| Chỉ rê chuột mà hình tự di chuyển | Nhấn Ctrl+Alt+Q; kiểm tra Drag Lock, Dwell Control và Zoom/Pan nếu đang bật trên iPhone, rồi thử lại |
| Chuột hoặc bàn phím bị giữ trong app | Nhấn Ctrl+Alt+Q; Esc là cách nhả bổ sung |
| Wireless không xuất hiện | Cùng mạng nội bộ, tránh mạng khách cô lập thiết bị; kiểm tra quyền mạng và runtime đi kèm |
| Báo thiếu DLL | Giải nén/cài lại đầy đủ gói. Không tải DLL rời từ website không rõ nguồn |
| Settings are malformed | Đóng app và giữ lại bản sao cấu hình để báo lỗi. Không ghi đè hoặc xóa cấu hình khi chưa sao lưu |

## 9. Gửi thông tin chẩn đoán

Mở **Settings → Advanced → Copy Diagnostics**. Khi báo lỗi, kèm phiên bản iMirror,
Windows, model/iOS, chế độ USB/Wireless và bước tái hiện. Kiểm tra nội dung trước
khi đăng công khai; không gửi Apple ID, mật khẩu, dữ liệu ghép đôi, UDID hoặc nội
dung màn hình riêng tư. Không bật tracing từng gói khi sử dụng thông thường.

Cấu hình nằm trong `%LOCALAPPDATA%\iMirror` (`settings.json` và `ble-input.json`).
Các số liệu decode/input là thời gian xử lý phần mềm, không phải độ trễ từ thao
tác tay đến hình trên iPhone. Source FPS khác tốc độ làm tươi màn hình Windows.

## 10. WDA nâng cao và gỡ cài đặt

WDA là tùy chọn cho người có runner WebDriverAgent đã được ký/cài trên iPhone,
Developer Mode và đường chuyển tiếp cục bộ phù hợp. Việc triển khai runner liên
quan Apple signing/Apple ID hoặc tài khoản của bên ký; iMirror không cung cấp
chứng chỉ. USB và Bluetooth Mouse thông thường không cần WDA. Xem
[hướng dẫn WDA nâng cao](docs/USER_GUIDE.md#advanced-wda).

Với bản đã cài: gỡ qua **Windows Settings → Apps → Installed apps → iMirror**.
Với thư mục thử nghiệm: đóng app rồi xóa thư mục đó. Cấu hình người dùng được giữ
riêng; sao lưu trước khi xóa `%LOCALAPPDATA%\iMirror` nếu muốn đặt lại hoàn toàn.
