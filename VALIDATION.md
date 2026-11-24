# Public release verification — 2026-11-16

## 3.3.0 — 2026-11-24

- `cargo test --locked --bin astrolight`: 26 passed.
- `node scripts/check-effects.cjs`: 52 modes, 624 cases (156 with per-key effects, all with key-press input), 132,912 channels, maximum difference 0.
- `node scripts/check-i18n.cjs`: 428 keys present in English, Vietnamese and Simplified Chinese.
- `npm test` in `scripts/`: 12 offline UI scenarios (effect selection, search and filters, per-key assignment, reactive preview, language switching, remap draft with undo/redo, macros, review before write, self-saving settings, 820 px window). No JavaScript errors.
- Caps Lock: a probe on Windows 11 confirmed that a worker thread without a message queue reads the correct toggle state after each synthesized press.
- Not verified on hardware in this pass: 3.3 lighting on the physical Lucky65, including per-key and reactive effects, skipped identical frames and the Caps Lock color. No remap or macro was written to a device.

## 3.2.0

The publication checkout passed 22 Rust unit tests, 336 JavaScript/Rust parity cases (71,568 channels, maximum difference 0), and the offline UI regression harness after a clean `npm ci`. Production Rust/UI inputs are unchanged from the validated 3.2.0 build. No remap/macro was written to hardware for publication.

Selected 3.2 results and the offline UI fixture are included in `validation/`. Older development measurements below are historical notes; their original raw logs are not included in this public checkout.

---

# Kiểm tra Astrolight 3.2 — 14/11/2026

- 22 unit test Rust đạt. Engine/HID, bộ ưu tiên gõ, encoder remap/macro giữ nguyên logic 3.1.
- Kiểm tra UI với dữ liệu mô phỏng: gán phím, Ctrl+C trên Fn 1, chuyển vị trí tự xóa modifier cũ, tìm “am luong”, tìm không có kết quả, tạo/gán macro, Undo/Redo, trạng thái sau lưu, hủy review không gọi ghi thiết bị, mũi tên chọn phím, roving focus và Reduced Motion.
- Sáu kích thước: 1440×960, 1280×920, 1120×760, 900×700, 720×600, 390×844. Cả năm trang không tràn ngang document; thư viện phím không bị thanh lưu che; phím remap nhỏ nhất trong các mẫu khoảng 29,9 px. Ở cửa sổ hẹp, bàn phím cuộn trong khung riêng. Ở cửa sổ thấp, trang cuộn thay vì ép thư viện vào vùng không đủ cao.
- Bảy cặp màu chữ/bề mặt được đo: contrast thấp nhất 6,71:1. Đây là kiểm tra các token và cặp màu chính, không phải audit WCAG đầy đủ.
- Release thật: 28 chế độ, tìm kiếm/bộ lọc, ưu tiên gõ, Undo/Redo và soạn remap/macro, save/review/cancel, rule foreground, backup roundtrip/từ chối backup lỗi, responsive. Không lỗi JavaScript. Đã giữ và khôi phục các tệp cài đặt của người dùng sau lượt kiểm tra.
- Không ghi remap/macro vào thiết bị trong lượt này. Chưa xác nhận hoạt động thực tế sau ghi hoặc độ bền cấu hình qua rút nguồn; giới hạn đọc ngược firmware vẫn như 3.1.

Cuối lượt release: 29.91 FPS USB, 8.03 ms/khung, không lỗi HID; preview rAF 165.01/giây. Các số này không thay cho phép đo quang học hoặc thử độ ổn định dài hạn.

Nguồn và quyết định thiết kế: [DESIGN-GUIDELINES.md](DESIGN-GUIDELINES.md).

---

## Lịch sử 3.1 và 3.0

# Kiểm tra Astrolight 3.1 — 14/11/2026

- 22 unit test Rust đạt: giữ/nhả nhiều phím, autorepeat, chống race khi đối chiếu trạng thái phím, khoảng nghỉ sau frame trễ, chuyển cấu hình 120 FPS cũ sang 30 FPS, encoder và cấu hình.
- 336 trường hợp đối chiếu preview JS với Rust (28 chế độ × 4 layout × 3 bộ tham số), 71.568 kênh màu: chênh lệch tối đa 0.
- Người dùng thử gõ nhanh/giữ rồi nhả, báo “Chưa thấy lỗi lặp/nhả chậm”. Các khoảng ưu tiên nhập liệu quan sát được không tăng số frame RGB, phase không chạy tiếp; sau nhả đèn chạy lại. Đây là kiểm tra ban đầu, chưa chứng minh loại bỏ lỗi trong mọi tình huống dài hạn.
- Bản release 3.1: kiểm tra 28 chế độ, 12 hiệu ứng mới, tìm kiếm không dấu/bộ lọc, tự kết nối/phát, bật/tắt ưu tiên gõ và lưu lại, remap/macro bản nháp, rule ứng dụng, backup roundtrip/từ chối dữ liệu lỗi, giao diện 390/720/1240 px. Không lỗi JavaScript. Không ghi remap/macro xuống thiết bị.
- Không còn lựa chọn 120 FPS. Mặc định 30 FPS; ưu tiên gõ bật mặc định. Giữ lệnh DIRECT mỗi frame vì thử chỉ gửi một lần khiến LED đứng màu.
- Chưa xác định chính xác cơ chế gây nhả phím chậm bên trong firmware. Các số đo 120 FPS bên dưới là lịch sử bản 3.0, không phải khuyến nghị hoặc bằng chứng nhập liệu ổn định.

Release cuối lượt: 29.94 FPS USB, 7.66 ms/khung; rAF preview 155.52/giây; lỗi HID: không. rAF là nhịp callback, không phải phép đo quang học.

---

## Lịch sử kiểm tra 3.0

# Kiểm tra Astrolight 3 — 14/11/2026

## Phạm vi và kết quả

- Lucky65 USB 0603:0351 trên máy Windows hiện tại, Tauri 2.12.5 / Rust 1.94. Không flash firmware.
- 12 unit test Rust đạt, gồm encoder RGB/remap/macro, giới hạn đầu vào, release macro, chuyển màu và rule ứng dụng. Hai report remap/macro 620 byte khớp hoàn toàn với fixture do mã gốc tạo ra.
- Bản release: tự kết nối và tự phát; chuyển qua 10 hiệu ứng mới không lỗi HID. UI có đủ 16 chế độ.
- Kiểm tra UI: logo loop, 3 layer và gán tổ hợp, tạo/sửa/ghi macro trong ô có focus, lưu bản nháp, màn hình review trước ghi, rule khớp tiến trình foreground thật và fallback về mặc định, backup roundtrip và từ chối dữ liệu lỗi. Không lỗi JavaScript.
- Hộp thoại Windows: xuất JSON ra tệp rồi chọn đúng tệp để nhập lại, qua màn hình xác nhận.
- Startup: bật, đọc lại trạng thái, tắt và đọc lại; để lại trạng thái ban đầu (tắt).
- Caps Lock: hook native nhận true → false → true khi thao tác bằng phím Windows thật; trả Caps về trạng thái ban đầu. Đây là kiểm tra tín hiệu và pipeline, không phải phép đo quang học LED Caps.
- Responsive: không tràn ngang ở 390, 720, 1240 px trên tất cả 5 tab; cửa sổ native đặt tối thiểu 720×600.

## Hiệu năng đo trên bản release

Sau kiểm tra chức năng và thêm 15 giây phát sóng liên tục:

| Chỉ số | Kết quả |
|---|---:|
| FPS USB cuối lượt | 120.078 |
| Thời gian gửi trung bình cuối lượt | 4.897 ms/khung |
| Tổng frame trong lượt | 3786 |
| Deadline trễ trong toàn lượt | 3 |
| Lỗi HID | Không |
| Callback preview rAF, đo 3 giây | 165.119/giây |

FPS USB không phải tốc độ PWM/quét LED trong firmware. rAF là nhịp callback vẽ, không đo quang học màn hình. Phép đo chỉ phản ánh máy/thiết bị hiện tại, có CDP gắn để kiểm tra.

## Giới hạn kiểm chứng

Không ghi đè remap/macro đang có của người dùng trong lượt test. Encoder và UI đã kiểm tra, nhưng thao tác remap/macro sau ghi và độ bền lưu sau ngắt nguồn **chưa xác nhận trên thiết bị**. Chưa xác minh Bluetooth/receiver 2.4 GHz. Startup chưa thử bằng đăng xuất/đăng nhập thực tế; đã kiểm tra giá trị đăng ký và trạng thái UI.

Lượt kiểm tra v2 trước đây có tổng private memory Astrolight + WebView2 khoảng 269 MiB khi vẽ 165 Hz với CDP; không dùng số này làm kết quả RAM v3. EXE nhỏ không bao gồm RAM hoặc runtime WebView2. Không khẳng định tổng RAM app chỉ vài MB.

Các JSON trong `validation/` chứa kết quả. Các phép đo v2 được giữ riêng để tham khảo, không phải kết quả bản 3.

## Thu nhỏ và khôi phục

Đã kiểm tra trên build debug của cùng thay đổi quản lý visibility: Windows báo minimized=true, giao diện nhận tín hiệu native để dừng vẽ, Rust vẫn chạy hiệu ứng Spiral. Trong 10 giây thu nhỏ gửi thêm 1216 frame; FPS cuối lượt 119.983; không lỗi HID. Khôi phục cửa sổ thành công. Bản release cuối đã chứa cùng mã xử lý. Không suy ra mức giảm RAM từ phép thử này.
