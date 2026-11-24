# Astrolight 3.3 — Lucky65 Studio

## Mới trong 3.3

- **Giao diện mới:** bàn phím là trung tâm, đèn LED hắt sáng qua khe phím và chữ trên phím; thư viện hiệu ứng có ảnh xem trước thật, chuyển động khi rê chuột. Cài đặt và profile ứng dụng tự lưu ngay khi thay đổi, không còn nút Lưu tùy chọn.
- **Đa ngôn ngữ:** English (mặc định), Tiếng Việt, 简体中文. Đổi ở thanh bên hoặc trong Cài đặt.
- **52 hiệu ứng:** thêm 18 hiệu ứng (xoay phổ màu, lăng kính, lốc xoáy, kính vạn hoa, hoa nở, mưa số, tia chớp, bóng nảy, đèn chạy, khảm màu, cuộn xoáy, zíc zắc, đèn dung nham, ánh nến, đèn chớp, cột nhạc, rắn săn mồi, lấp lánh) và 6 hiệu ứng theo phím bấm (chớp sáng, tỏa sáng, gợn sóng, chữ thập, bản đồ nhiệt, cầu vồng lan tỏa).
- **Hiệu ứng từng phím:** chuyển sang **Từng phím**, chọn phím (bấm, kéo, hoặc bộ chọn nhanh), rồi chọn hiệu ứng, màu và tốc độ riêng cho các phím đó. Phần còn lại giữ hiệu ứng của bàn phím.
- **Hiệu năng:** không gửi khung USB khi không có LED nào đổi màu; sơ đồ phím chỉ đọc một lần; bản xem trước dừng hẳn khi không cần vẽ; không gửi trạng thái sang giao diện khi thu nhỏ.
- **Caps Lock:** đọc trạng thái thật từ Windows ở mỗi khung hình thay vì dự đoán trong keyboard hook, và tùy chọn có hiệu lực ngay khi bật.

Các mục bên dưới mô tả bản 3.2; tên nút trong app thay đổi theo ngôn ngữ đang chọn.

Ứng dụng Windows viết bằng **Tauri + Rust**, UI HTML/CSS/Canvas thuần. Không đóng gói Node.js/Chromium; sử dụng WebView2 của Windows. Toàn bộ UI và logo nằm trong EXE.

## Dùng ngay

Mở **Astrolight.exe**, kết nối Lucky65 bằng USB. App tự tìm thiết bị `0603:0351`, phát intro trên bàn phím khoảng 2,8 giây, chuyển mượt về hiệu ứng/profile đã chọn và tiếp tục phát. Nút **Phát / Dừng** luôn ở thanh trên. Thu nhỏ cửa sổ vẫn chạy đèn; đóng app sẽ dừng luồng RGB.

## Những gì có trong bản 3.2

- Giao diện tối, bố cục co giãn, logo Astrolight SVG có chuyển động lặp. Có tùy chọn tắt chuyển động logo; tôn trọng Reduced Motion của Windows.
- **28 chế độ:** tô từng phím, một màu, gradient, sóng, nhịp thở, cầu vồng, cực quang, vòng xoáy, gợn nước, sao băng, sao lấp lánh, mưa màu, lửa, giao thoa, hải đăng, hoàng hôn, Plasma, tinh vân, đại dương, mưa sao băng, đom đóm, sóng chữ V, kim cương, xoắn kép, dải lụa, máy quét, nhịp tim, chong chóng. Có tìm kiếm không dấu và bộ lọc Mới / Thư giãn / Hình học / Năng lượng / Tĩnh. Thêm nút duyệt hiệu ứng tự động mỗi 5 giây, màu A/B, độ sáng, tốc độ, hướng và độ rộng. Tác dụng hướng/độ rộng tùy công thức của hiệu ứng.
- 19 hiệu ứng firmware từ bản gốc. Firmware chạy độc lập; chế độ này tạm nhường điều khiển đèn cho bàn phím, không áp dụng Caps/profile ứng dụng.
- **Gán phím theo bố cục VIA:** chọn phím trên hình, chọn mã phím ở bảng bên dưới. Có Base, Fn 1, Fn 2; phím cơ bản, media, chuột, Fn, tổ hợp Ctrl/Shift/Alt/Win + một phím và macro. Đây là giao diện tham khảo VIA dùng giao thức Lucky65, không phải firmware QMK/VIA.
- **Macro trên bàn phím:** tối đa 10 slot, 84 sự kiện mỗi macro. Soạn thủ công hoặc ghi trong ô có focus; sửa delay từng sự kiện, số lần lặp 1–1000. Kiểm tra cặp nhấn/nhả và delay 20–32767 ms trước khi ghi. Recorder không chạy nền, không ghi khi ra khỏi ô và không lưu lịch sử gõ ngoài macro.
- **Intro LED:** chọn hiệu ứng và thời lượng 0,5–10 giây. Chạy một lần khi kết nối lần đầu trong phiên; sau đó chuyển về profile. Nếu tắt tiếp tục phát, app chuyển về một khung của profile rồi dừng. Tự nối lại trong cùng phiên không lặp intro.
- **Caps Lock:** màu khi bật, màu khi tắt hoặc trở về hiệu ứng nền. Theo trạng thái Caps Lock của Windows, áp dụng lên vị trí Caps vật lý, có tính độ sáng chung. Hoạt động trong luồng đèn tùy chỉnh của app.
- **Profile theo app/game:** theo ứng dụng đang ở phía trước, kiểm tra mỗi 500 ms. Match tên file `.exe`, không phân biệt hoa/thường; rule trên cùng ưu tiên. Không match thì trở về hiệu ứng mặc định. Có thể chọn EXE qua hộp thoại và lấy preset hoặc hiệu ứng đang chỉnh. Các rule lưu bản sao hiệu ứng, không tự thay đổi theo preset sửa sau đó.
- **Import/export toàn bộ:** ánh sáng, preset, rule ứng dụng, remap, macro và tùy chọn intro/Caps/logo. Xem trước nội dung trước khi nhập. Startup không được thay đổi bởi backup.
- **Startup Windows:** tùy chọn trong Cài đặt; áp dụng ngay cho tài khoản hiện tại qua HKCU Run. Để EXE tại vị trí cố định trước khi bật; nếu chuyển chỗ, tắt/bật lại tùy chọn. Mặc định tắt.

## Giao diện 3.2

Thiết kế lại theo tài liệu Microsoft, NN/g, W3C và tham khảo tương tác VIA. Nguồn, tác giả/tổ chức và cách áp dụng được ghi trong [DESIGN-GUIDELINES.md](DESIGN-GUIDELINES.md). Đây là triển khai riêng, không phải thiết kế được các tổ chức đó thẩm định.

- Vùng bàn phím và inspector đặt cạnh nhau; ở cửa sổ hẹp tự xếp lại. Màn remap dùng keycap trung tính, không vẽ animation RGB trên canvas.
- Chọn vị trí bằng sơ đồ hoặc danh sách; phím được tùy chỉnh hiển thị cả chức năng và vị trí gốc. Phím ẩn trong layout vẫn có trong bản nháp nhưng phải chọn layout phù hợp để thao tác trên sơ đồ.
- Thư viện chức năng có nhóm, tìm kiếm xuyên nhóm với tên tiếng Việt không dấu; tìm cả macro. Modifier được đồng bộ theo phím đang chọn, tránh vô tình giữ Ctrl/Alt từ vị trí trước.
- **Hoàn tác / Làm lại** tối đa 60 trạng thái bản nháp trong phiên, gồm remap và macro. Ctrl+Z / Ctrl+Shift+Z khi đang ở trang Gán phím và không gõ trong ô nhập. Mũi tên di chuyển giữa các phím trên sơ đồ; Enter/Space chọn.
- Trạng thái phân biệt **chưa lưu / đã lưu trên máy / khớp lần gửi trong phiên**. App không suy diễn rằng bản nháp đang khớp firmware khi mới mở.
- **Xem trước & ghi…** mở màn review trước khi ghi. Lưu bản nháp không ghi xuống thiết bị. Nếu đóng app mà chưa lưu, các sửa đổi bản nháp trong phiên chưa được lưu lại.
- Giữ nguyên 28 chế độ đèn và engine 3.1: mặc định 30 FPS, ưu tiên gõ phím bật.

## Remap/macro: bản nháp và bản trên bàn phím

1. Chọn Base/Fn và phím cần gán, rồi chọn chức năng. Có thể thêm modifier cho phím cơ bản.
2. Tạo macro trong tab Macro, sau đó chọn macro ở bảng gán phím.
3. **Lưu bản nháp** chỉ lưu trên máy.
4. **Xem trước & ghi…** hiển thị danh sách thay đổi trước khi ghi cả ba bảng phím và macro qua HID.

App **chưa đọc ngược keymap/macro đang có trong firmware**. Vì vậy lần đầu sẽ hiển thị mặc định hoặc bản từng lưu trong Astrolight, không phải bản chụp cấu hình thực tế của bàn phím. Nút ghi thay thế cấu hình bàn phím bằng bản này. Nhập backup không tự ghi remap/macro vào thiết bị. “Mặc định” là hành vi mặc định của firmware, không phải transparent layer QMK.

Encoder remap/macro đã được đối chiếu từng byte với hàm gốc của hãng (fixture test). Bản 3 chưa được xác nhận thao tác gán phím/macro thực tế sau ghi trên thiết bị; lượt kiểm tra phát hành không ghi đè keymap hiện tại của người dùng. Xem `VALIDATION.md`.

## Độ mượt và bộ nhớ

Luồng Rust riêng sở hữu HID, mặc định **30 FPS**; có 15 FPS và 60 FPS (tải USB cao hơn). Profile 120 FPS cũ tự chuyển về 30 FPS khi sử dụng. Bản 3.0 từng gửi 120 FPS nhưng người dùng gặp nhả phím chậm/lặp ký tự; số khung gửi thành công không chứng minh firmware xử lý phím ổn định. Bản 3.1 giữ lệnh DIRECT mỗi frame, nghỉ 2 ms sau lệnh và ít nhất 0,5 ms giữa các gói; luôn có ít nhất 8 ms nghỉ sau cả frame, không gửi dồn để bù deadline. Không xếp hàng khung cũ; cấu hình slider dùng mailbox mới nhất. Màu/gói HID nằm trên stack; hình học và seed hiệu ứng được tính trước, không clone cấu hình hoặc parse hex mỗi frame. Chuyển cảnh 180 ms.

**Ưu tiên gõ phím bật mặc định:** giữ nguyên LED khi một phím đang được giữ và trong 180 ms sau sự kiện phím gần nhất; sau đó chuyển mượt và chạy tiếp. Có ô thử gõ và trạng thái trên thanh đầu. Điều này áp dụng cả khi chơi game, mở app khác hoặc thu nhỏ Astrolight; đèn custom sẽ đứng màu khi giữ WASD. Có thể tắt trong Cài đặt nhưng sẽ bỏ cơ chế giảm xung đột này. Nếu Windows không mở được bộ theo dõi phím, luồng custom tạm dừng thay vì gửi liên tục. Một frame đang gửi sẽ hoàn tất, không cắt giữa 5 gói.

Preview dùng Canvas và requestAnimationFrame theo tần số màn hình. Logo dùng CSS transform/opacity, không video hay thư viện animation. Cảnh tĩnh không vẽ/gửi lặp khung thừa. Preview tạm dừng khi ẩn hoặc mở tab không có bàn phím. WebView2 nhận memory target Low khi thu nhỏ; mức RAM thực tế phụ thuộc runtime và GPU, không thể suy ra từ kích thước EXE.

FPS USB là số khung gửi thành công, không phải tần số PWM/quét LED trong firmware. Không cung cấp 120/144 FPS; ưu tiên độ ổn định nhập liệu. Đây là biện pháp tránh xung đột quan sát được; chưa xác định chính xác cơ chế lỗi bên trong firmware.

## Dữ liệu và riêng tư

Thư mục `%APPDATA%\local.astrolight.lucky65`:

| Tệp | Nội dung |
|---|---|
| `settings.json` | Ánh sáng mặc định, lưu khi đóng bình thường |
| `options.json` | Intro, Caps, ưu tiên gõ, logo, rule ứng dụng |
| `keymap.json` | Bản nháp remap và macro |
| `presets.json` | Thư viện preset |

Giữ khả năng nhập preset đèn từ Astrolight 2/Electron v1. Preset cũ được sao chép lần đầu, không sửa tệp gốc.

Không telemetry hay tải nội dung mạng trong UI. Profile ứng dụng chỉ đọc tên tiến trình foreground. Hook phím lưu trạng thái nhấn/nhả tạm thời và mốc thời gian để ưu tiên nhập liệu, đồng thời theo dõi Caps Lock; không lưu nội dung gõ hay lịch sử phím, không chặn/sửa/giả lập sự kiện bàn phím. Macro recorder chỉ hoạt động trong ô ghi có focus. App không tự chạy game/EXE đã chọn.

## Build

Cần Rust MSVC, C++ Build Tools, WebView2. Không cần npm để build app.

```powershell
.\Build.ps1
# dist\Astrolight.exe
cd src-tauri
cargo test --locked --bin astrolight
cargo run --locked --bin astrolight
```

Đối chiếu preview JS và màu Rust, không chạm thiết bị: `node scripts/check-effects.cjs`. Cần Node.js chỉ cho phép kiểm tra này.

Đo HID (đổi màu bàn phím ở 15/30 FPS; đóng phần mềm RGB khác trước; benchmark không có bộ ưu tiên gõ phím):

```powershell
cargo run --locked --release --bin hid-benchmark -- --write-rgb --seconds 10
```

Các tệp chính: `effects.rs` (52 hiệu ứng), `engine.rs` (lịch RGB/intro/profile/Caps), `keymap.rs` (remap/macro), `system.rs` (Windows), `settings.rs` (tùy chọn/rule), `main.rs` (IPC và lưu trữ), `ui/app.js` (UI), `ui/i18n.js` (ngôn ngữ), `ui/effects.js` (preview). `PROTOCOL-v3.md` ghi chi tiết giao thức.

## Giới hạn

Chỉ kiểm tra USB Lucky65 `0603:0351`, chưa kiểm tra Bluetooth/2.4 GHz hoặc độ bền lưu cấu hình sau rút nguồn. Một số game được bảo vệ có thể không cho đọc tên tiến trình; khi không đọc được, app về profile mặc định. Hook Caps có thể bị Windows/phần mềm bảo vệ hạn chế; UI báo khi không khởi tạo được. Không mở đồng thời phần mềm RGB hãng để tránh hai app tranh điều khiển.

Ý tưởng hiệu ứng tham khảo [QMK RGB Matrix](https://docs.qmk.fm/features/rgb_matrix), quy trình chọn phím tham khảo [VIA](https://github.com/the-via/app); công thức và UI được triển khai riêng. Tham khảo kỹ thuật: [Tauri](https://v2.tauri.app/start/prerequisites/), [hidapi](https://docs.rs/hidapi/latest/hidapi/struct.HidDevice.html), [Windows foreground](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getforegroundwindow), [Windows Startup](https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys).

## Kiểm tra UI offline

Trong thư mục source: `cd scripts`, `npm ci`, `npm test`, rồi `cd ..` (Node.js 20+ và Microsoft Edge). Harness dùng Microsoft Edge headless, dữ liệu mô phỏng và HTTP loopback tạm thời; không truy cập HID, không thay cấu hình người dùng. Ảnh/kết quả ghi vào `validation/ui-run/`. Các dependency Node chỉ dành cho kiểm tra, không nằm trong app EXE.
