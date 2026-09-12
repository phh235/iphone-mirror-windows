//! App-owned UI text. Device names, diagnostics JSON keys and driver errors stay intact.
use imirror_device::Language;
use std::cell::Cell;
thread_local! { static LANGUAGE:Cell<Language>=const { Cell::new(Language::English) }; }
pub fn set(language: Language) {
    LANGUAGE.set(language);
}
pub fn language() -> Language {
    LANGUAGE.get()
}
pub fn tr(english: &str) -> &str {
    if language() == Language::English {
        return english;
    }
    match english {
        "General" => "Chung",
        "Language" => "Ngôn ngữ",
        "Choose the application language." => "Chọn ngôn ngữ hiển thị của ứng dụng.",
        "Changes apply immediately and are saved automatically." => {
            "Áp dụng ngay và tự động lưu lựa chọn."
        }
        "iMirror Settings" => "Cài đặt iMirror",
        "Connection" => "Kết nối",
        "Control" => "Điều khiển",
        "Display" => "Hiển thị",
        "Advanced" => "Nâng cao",
        "Choose how your iPhone connects." => "Chọn cách kết nối iPhone.",
        "Mouse and keyboard control for iPhone." => "Điều khiển iPhone bằng chuột và bàn phím.",
        "Choose how the image fits your window." => "Chọn cách hiển thị hình trong cửa sổ.",
        "Optional tools and troubleshooting." => "Công cụ tùy chọn và xử lý sự cố.",
        "Connection mode" => "Chế độ kết nối",
        "Automatic" => "Tự động",
        "Wireless" => "Không dây",
        "Device" => "Thiết bị",
        "Refresh" => "Làm mới",
        "Receiver name" => "Tên bộ nhận",
        "Enable Control" => "Bật điều khiển",
        "Input mode" => "Chế độ điều khiển",
        "Bluetooth Mouse" => "Chuột Bluetooth",
        "Advanced automation (WDA)" => "Điều khiển nâng cao (WDA)",
        "Pointer sensitivity" => "Độ nhạy chuột",
        "Release shortcut" => "Phím nhả điều khiển",
        "Status" => "Trạng thái",
        "Scaling" => "Tỷ lệ hiển thị",
        "Fit" => "Vừa khung",
        "Fill" => "Lấp đầy",
        "Show the entire iPhone screen." => "Hiển thị toàn bộ màn hình iPhone.",
        "One source pixel per screen pixel." => "Một pixel nguồn ứng với một pixel hiển thị.",
        "Fill the window; edges may be cropped." => "Lấp đầy cửa sổ; các mép có thể bị cắt.",
        "Rendering" => "Trình bày hình",
        "Synchronized display" => "Đồng bộ hiển thị",
        "Advanced features" => "Tính năng nâng cao",
        "Enable advanced features" => "Bật tính năng nâng cao",
        "Troubleshooting" => "Xử lý sự cố",
        "Diagnostics" => "Chẩn đoán",
        "Copy Diagnostics" => "Sao chép chẩn đoán",
        "Connect WDA" => "Kết nối WDA",
        "Requires a signed iPhone runner, Developer Mode and a local connection. Optional." => {
            "Cần runner đã ký trên iPhone, Developer Mode và kết nối cục bộ. Không bắt buộc."
        }
        "Done" => "Xong",
        "No iPhone detected" => "Chưa tìm thấy iPhone",
        "Connected" => "Đã kết nối",
        "Connecting…" => "Đang kết nối…",
        "Connection unavailable — open Settings" => "Lỗi kết nối — mở Cài đặt",
        "Not connected" => "Chưa kết nối",
        "Connect" => "Kết nối",
        "Disconnect" => "Ngắt kết nối",
        "Rotate" => "Xoay",
        "Home" => "Màn hình chính",
        "Settings" => "Cài đặt",
        "Fullscreen" => "Toàn màn hình",
        "Exit fullscreen" => "Thoát toàn màn hình",
        "More actions" => "Thao tác khác",
        "Control on" => "Đã bật điều khiển",
        "Control active" => "Đang điều khiển",
        "Control iPhone" => "Điều khiển iPhone",
        "Control active — Ctrl+Alt+Q to release" => "Đang điều khiển — Ctrl+Alt+Q để nhả",
        "Control iPhone — waiting for connection" => "Điều khiển iPhone — đang chờ kết nối",
        "Connect your iPhone" => "Kết nối iPhone của bạn",
        "Connect your iPhone by USB and unlock it" => "Cắm cáp USB và mở khóa iPhone",
        "Connect your iPhone by USB and unlock it.\nChoose Wireless in Settings to connect over Wi-Fi." => {
            "Cắm cáp USB và mở khóa iPhone.\nChọn Không dây trong Cài đặt để dùng Wi-Fi."
        }
        "On iPhone, open Screen Mirroring and choose this PC." => {
            "Mở Phản chiếu màn hình trên iPhone và chọn PC này."
        }
        "A connection operation is in progress" => "Đang thực hiện thao tác kết nối",
        "Control is off." => "Điều khiển đang tắt.",
        "Advanced control connected." => "Đã kết nối điều khiển nâng cao.",
        "Advanced control is not ready. Start the signed runner and local connection; see Diagnostics." => {
            "Chưa sẵn sàng. Khởi chạy runner đã ký và kết nối cục bộ; xem Chẩn đoán."
        }
        "Ready. Click the iPhone screen. Ctrl+Alt+Q releases control." => {
            "Sẵn sàng. Bấm vào hình iPhone để điều khiển; Ctrl+Alt+Q để nhả."
        }
        "Bluetooth control is unavailable on this adapter. See Diagnostics." => {
            "Adapter không hỗ trợ điều khiển Bluetooth. Xem Chẩn đoán."
        }
        "Turn on Windows Bluetooth." => "Hãy bật Bluetooth trên Windows.",
        "Pair this PC in iPhone Bluetooth settings. Enable AssistiveTouch." => {
            "Ghép đôi PC trong cài đặt Bluetooth iPhone và bật AssistiveTouch."
        }
        "Starting Bluetooth control. See Diagnostics if it does not start." => {
            "Đang bật điều khiển Bluetooth. Xem Chẩn đoán nếu không khởi động được."
        }
        "Mouse control is unavailable. Open Advanced Diagnostics for details; Ctrl+Alt+Q must be available before capture." => {
            "Không thể điều khiển chuột. Xem Chẩn đoán nâng cao; Ctrl+Alt+Q phải khả dụng trước khi thu nhận input."
        }
        "Could not save settings" => "Không lưu được cài đặt",
        "Could not copy diagnostics" => "Không sao chép được chẩn đoán",
        "iMirror could not start" => "Không khởi động được iMirror",
        "Advanced Diagnostics" => "Chẩn đoán nâng cao",
        "Move Right" => "Di chuyển phải",
        "Move Left" => "Di chuyển trái",
        "Left Click" => "Bấm trái",
        "Type A" => "Gõ A",
        "Mouse speed" => "Độ nhạy chuột",
        "Layout test — no phone video" => "Kiểm tra bố cục — không có video iPhone",
        _ => english,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn languages_switch_without_translating_arbitrary_data() {
        set(Language::Vietnamese);
        assert_eq!(tr("Settings"), "Cài đặt");
        assert_eq!(tr("My iPhone"), "My iPhone");
        set(Language::English);
        assert_eq!(tr("Settings"), "Settings");
    }
}
