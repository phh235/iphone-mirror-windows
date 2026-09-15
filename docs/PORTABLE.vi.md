# iMirror — bắt đầu trên Windows

[English](PORTABLE.en.md) · [WDA từng bước](WDA_SETUP.vi.md)

## 1. Tải và mở app

Tải **iMirror-v0.1.0-windows-x64.zip** từ Release của
[phh235/iphone-mirror-windows](https://github.com/phh235/iphone-mirror-windows/releases).
Chọn **Extract All / Giải nén tất cả**, vào một thư mục cố định, chẳng hạn
`Documents\iMirror`. Mở `START-HERE.html` để đọc hướng dẫn offline, hoặc mở
`iMirror.exe` để chạy. Không chạy EXE ngay trong ZIP và không chép riêng EXE.

Gói gồm DLL USB, runtime Microsoft, thư mục AirPlay, WDA và giấy phép. Không cần
Rust, Git, Python, Node, Visual Studio hoặc FFmpeg CLI. Đây là **bản thử nghiệm
chưa ký số**, chưa xác nhận Windows sạch; không phải bản bảo đảm cho mọi máy.
Nếu Windows chặn, kiểm tra đúng trang tải và hash SHA256 của Release trước;
không tắt Defender hoặc cài DLL từ website lạ.

Nhấn chuột phải EXE → Show more options → Send to → Desktop để tạo shortcut
nếu muốn. Bản ZIP không tự tạo mục Start Menu hoặc mục gỡ cài đặt.

## 2. Chọn cách sử dụng

- **USB + Bluetooth Mouse:** lựa chọn đầu tiên cho mirror và điều khiển bằng
  con trỏ AssistiveTouch. Không cần đăng nhập Apple ID, Developer Mode, jailbreak
  hoặc app cài trên iPhone.
- **Wireless + Bluetooth Mouse:** hình qua mạng, chuột qua Bluetooth.
- **Wireless + WDA qua USB:** click theo vị trí trên hình, cần thiết lập nâng cao
  trong [hướng dẫn WDA](WDA_SETUP.vi.md). Vuốt được thực hiện sau khi thả chuột;
  chưa phải kéo bám tay realtime. WDA không bắt buộc để dùng app.

App hiện không phát audio. Độ phân giải và FPS tùy nguồn và máy; không bảo đảm 60 FPS.

## 3. Mirror USB

1. Dùng Windows 11 x64, hoặc Windows 10 22H2 khi driver/API hỗ trợ. Dùng cáp truyền
   dữ liệu, cắm iPhone, mở khóa và bấm **Tin cậy máy tính này** nếu hiện.
2. Nếu Windows chưa nhận iPhone, cài Apple Devices từ
   [hướng dẫn tải chính thức của Apple](https://support.apple.com/en-us/118290).
   Gói iMirror không chứa driver Apple. Apple Devices nhận máy chưa bảo đảm mọi
   máy sẽ hỗ trợ capture USB. Không dùng Zadig đổi driver USB cha của iPhone.
3. Mở iMirror → **Settings → Connection → USB**. Chọn thiết bị, Refresh nếu cần.
   Đóng Settings rồi bấm biểu tượng Connect. Automatic cũng có thể tự kết nối.
4. Hình phải cập nhật khi bạn thao tác trực tiếp trên iPhone. Cửa sổ Fit tự theo
   tỷ lệ màn hình. Fullscreen có thể có viền đen để giữ đúng tỷ lệ.

iPhone có thể tạm biến mất khỏi This PC khi chuyển chế độ capture. Khi đóng app,
đợi tiến trình hoàn tất khôi phục USB trước khi mở lại hoặc xóa thư mục.

## 4. Bluetooth: ghép đôi và bật điều khiển

1. Bật Bluetooth trong Windows. Trong iMirror chọn **Settings → Control**,
   chọn **Automatic** hoặc **Bluetooth Mouse**, bật **Enable Control**.
2. Chờ app báo sẵn sàng ghép đôi. Nếu adapter không hỗ trợ BLE peripheral, máy
   không thể dùng chế độ này dù vẫn kết nối tai nghe được; mirror vẫn dùng được.
3. Trên iPhone vào **Cài đặt → Trợ năng → Cảm ứng → AssistiveTouch**, bật
   AssistiveTouch. Vào **Thiết bị → Thiết bị Bluetooth**, chọn tên PC mà iMirror
   quảng bá; không mặc định tìm tên PHH235 trên mọi máy.
4. Hoàn tất thông báo ghép đôi trên cả iPhone và Windows. Chờ iMirror báo
   **Ready / Sẵn sàng**. Bluetooth báo Connected chưa có nghĩa điều khiển đã sẵn sàng.
5. Click bên trong hình mirror để giữ chuột/bàn phím cho iPhone. Di chuyển, click,
   giữ nút trái và kéo, hoặc lăn bánh xe. Gõ vào ô nhập liệu đang được chọn trên iPhone.
6. **Ctrl+Alt+Q** trả điều khiển về Windows. Esc cũng có thể nhả capture. Chỉ mở
   một iMirror để tránh tranh phím tắt. Đừng đánh giá readiness từ chữ Connected của USB.

Đường dẫn ghép chuột được Apple mô tả trong
[hướng dẫn thiết bị trỏ](https://support.apple.com/en-us/111775).
Bluetooth Mouse dùng chuyển động tương đối: click tác động ở con trỏ iPhone,
không bảo đảm trùng tuyệt đối con trỏ Windows. Chỉnh sensitivity trong Settings
và tracking speed trên iPhone cho phù hợp. Không cần bật Zoom. Nếu tự kéo/di chuyển
màn hình khi rê, nhả capture rồi kiểm tra Drag Lock, Dwell và Zoom nếu đang bật.

Nếu không Ready: giữ app mở, kiểm tra Bluetooth và AssistiveTouch; xem
**Settings → Advanced → Diagnostics**. Chỉ khi ghép đôi cũ lỗi, Forget tên PC trên
iPhone và Remove đúng iPhone trong Windows rồi ghép lại từ AssistiveTouch. Không
xóa ghép đôi của thiết bị khác. Không có subscriber thì app không gửi điều khiển.

## 5. Mirror Wireless

Đặt PC/iPhone cùng mạng nội bộ. Chọn **Settings → Connection → Wireless**, đặt tên
receiver, đóng Settings và Connect. Trên iPhone mở **Trung tâm điều khiển → Phản
chiếu màn hình** và chọn receiver. Cho phép mạng riêng nếu Windows hỏi; không tắt
firewall toàn bộ. Mạng khách/AP isolation có thể chặn discovery.

Giữ thư mục AirPlay nguyên vẹn. Nếu hình đứng, Disconnect/Connect và chọn lại
receiver. Chất lượng cao tăng lượng dữ liệu; FPS nguồn thực tế khác FPS được yêu cầu.
Với WDA, dùng Wireless cho hình và giữ cáp USB dành cho điều khiển.

## 6. Ngôn ngữ, cập nhật và hỗ trợ

**Settings → General → Language** cho phép đổi English/Tiếng Việt. Tooltip giải
thích các icon; ở cửa sổ hẹp một số lệnh nằm trong menu dấu ba chấm.

Để cập nhật, đóng app, đợi helper dừng rồi giải nén gói mới vào thư mục mới. Không
trộn DLL của hai phiên bản. Cấu hình nằm riêng trong `%LOCALAPPDATA%\iMirror` và
được giữ khi thay thư mục app. Để gỡ bản ZIP, đóng app rồi xóa thư mục đã giải nén;
chỉ xóa cấu hình riêng khi muốn reset và đã sao lưu dữ liệu WDA cần thiết.

Nếu lỗi: dùng **Settings → Advanced → Copy Diagnostics**, ghi rõ phiên bản, kiểu
kết nối và bước tái hiện. Che thông tin riêng tư trước khi đăng issue; không gửi
Apple ID, mật khẩu, 2FA, UDID, pairing records hoặc file đã ký. Không chép thư mục
cấu hình của người khác vào máy mình. Mọi thiết bị WDA phải thiết lập riêng.
