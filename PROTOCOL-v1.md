# Lucky65 3M — ghi chép giao thức RGB

Phân tích ngày 14/11/2026, trên bản cài tại `C:\Program Files (x86)\Lucky65_3M RGB Mechanical Keyboard` và bàn phím USB đang kết nối. Mục tiêu là tương tác điều khiển ánh sáng.

## Kiến trúc phần mềm gốc

Đây là ứng dụng Electron có giao diện Angular, không phải chỉ một kernel driver riêng. `resources/app.asar` chứa JavaScript và các addon Node native. `package.json` của ứng dụng ghi version 1.0.0.

Tệp `version` cạnh EXE ghi Electron 4.0.0. Dấu vân tay của mẫu được phân tích:

```text
app.asar SHA-256
c7c6544c910c22cabe4fb02d5bc7d6af01ec060508383b9bd8a322e360ec80be

x64/hiddevice.node SHA-256
b5a4a3900ff22f06e42d1d7767787654fe9bb1d7a948e0a137f308bcd6dbd133
```

Luồng giao tiếp liên quan đến RGB:

```text
Giao diện Angular
  → backend/protocol/device/keyborad/JupengSeries.js
  → backend/protocol/device/Device.js
  → backend/protocol/nodeDriver/HID.js
  → x64/hiddevice.node
  → HIDAPI / Windows HID
  → firmware Lucky65
```

Phần JavaScript bị đổi tên biến và giấu chuỗi bằng bảng tra; có thể giải bảng chuỗi mà không chạy ứng dụng gốc. Bảng LED nằm trong `backend/others/SKLocation.js`; bố cục phím trong `components/KeyBoardStyle.js`.

App mới tự xây gói lệnh bằng JavaScript, dùng `node-hid` để gọi Windows HID. Không sao chép giao diện, ảnh hoặc addon nhị phân của hãng vào bản phát hành.

## Thiết bị và HID collections

Thiết bị thực tế: vendor `JP`, product `lucky65`, USB `VID 0x0603`, `PID 0x0351`. `node-hid` báo release decimal 1319 (`0x0527`); không suy diễn đây là phiên bản firmware dạng chuỗi.

Độ dài được đọc trực tiếp từ `HidD_GetPreparsedData` / `HidP_GetCaps` trên thiết bị:

| Usage page | Usage | Input | Output | Feature | Vai trò |
|---|---|---:|---:|---:|---|
| `0xFF01` | `0x0001` | 9 | 9 | 0 | Ngoài phạm vi RGB của app mới |
| `0xFF55` | `0x0202` | 64 | 64 | 0 | Khung RGB trực tiếp |
| `0xFF02` | `0x0001` | 0 | 0 | 620 | Hiệu ứng firmware / màu tùy chỉnh |
| `0xFF03` | `0x0001` | 0 | 0 | 8 | Lệnh điều khiển ngắn |

**Độ dài bao gồm report ID ở byte 0. Không thêm byte 0 hoặc report ID lần thứ hai.**

Trong addon native x64 đã phân tích:

- `SetFeatureReport_Nolength`, địa chỉ `0x180004590`: lấy kích thước report đã lưu, sao chép buffer từ byte đầu, truyền nguyên buffer vào `hid_send_feature_report`.
- `SetHidWrite_Nolength`, địa chỉ `0x180003CB0`: sao chép nguyên buffer rồi gọi `hid_write`.
- Hai hàm export HIDAPI tương ứng nằm ở `0x180007C20` và `0x1800078D0`.

Điều này giải thích vì sao mã gốc tạo buffer 620 byte cho lệnh ngắn nhưng trên dây chỉ gửi 8 byte: addon dùng kích thước của HID collection, không dùng chiều dài buffer JavaScript.

## Chế độ RGB trực tiếp

Mỗi khung được gửi theo thứ tự:

1. Feature report 8 byte trên collection `FF03:0001`:

   ```text
   08 06 00 00 00 00 00 00
   ```

2. Chờ 10 ms.
3. Gửi 5 output report trên `FF55:0202`, mỗi report dài 64 byte:

   | Offset | Giá trị |
   |---:|---|
   | 0 | Report ID `0x06` |
   | 1 | Số thứ tự gói, từ 1 đến 5 |
   | 2–63 | 62 byte của luồng màu |

4. Chờ 5 ms sau gói thứ nhất; 10 ms sau mỗi gói còn lại, theo mã gốc.

Luồng màu là RGB xen kẽ: `R0 G0 B0 R1 G1 B1 ...`. Ma trận có 102 vị trí, cần 306 byte; 4 byte cuối của tổng 310 byte truyền bằng 0. Một bộ RGB có thể bị chia giữa hai gói, nên phải chia luồng byte sau khi tạo ma trận.

Ma trận không đánh số theo thứ tự các phím được vẽ. Ví dụ:

| Phím | Chỉ số ma trận | Offset RGB trong luồng |
|---|---:|---:|
| Esc | 17 | 51 |
| Q | 35 | 105 |
| A | 52 | 156 |
| Space dài (`mspace`) | 91 | 273 |
| Mũi tên phải | 99 | 297 |

`src/keyboard.json` giữ đầy đủ ánh xạ. Danh sách gốc có 71 slot phím cho các biến thể bố cục; ANSI Space dài hiển thị 67 phím. Những slot không tồn tại trong danh sách giữ màu 0.

App mới xếp hàng thao tác HID và chỉ tạo khung tiếp theo khi khung trước đã hoàn tất, không tích hàng đợi vô hạn. Sau mỗi khung có khoảng nghỉ thêm 45 ms; tốc độ thực tế phụ thuộc Windows/USB. Thay đổi tham số hiệu ứng được gộp trong 70 ms ở giao diện.

**Đã xác minh:** dữ liệu khớp mã gốc; gửi thật thành công; người dùng xác nhận đúng dải màu trên thiết bị.

## Hiệu ứng trong firmware

Feature report trên `FF02:0001`, 620 byte, các byte không nêu bằng 0:

| Offset | Ý nghĩa |
|---:|---|
| 0 | `0x07` — report ID |
| 1 | `0x02` — thiết lập hiệu ứng |
| 2 | `0x01` |
| 9 | ID hiệu ứng 0–19 |
| 10 | Độ sáng 0–4 |
| 11 | Tốc độ 1–5 |
| 12–31 | Chỉ số bảng màu cho 20 hiệu ứng |

Mã gốc dùng chỉ số 7 cho nhiều màu (ngoại trừ xử lý riêng hiệu ứng Fixed_on). Bảng màu cố định chính xác chưa được ánh xạ theo tên RGB trong app mới; phần firmware dùng chỉ số 7 như bảng màu mặc định. Muốn RGB bất kỳ, dùng direct mode hoặc custom mode.

| ID | Tên trong phần mềm gốc |
|---:|---|
| 0 | Neon_stream |
| 1 | Fixed_on |
| 2 | Respire |
| 3 | Ripples_shining |
| 4 | Rainbow_wheel |
| 5 | RippleBandUpDown |
| 6 | Reaction |
| 7 | TwoBlock |
| 8 | RandomColor |
| 9 | DoubleWave |
| 10 | RetroSnake |
| 11 | DoubleSpiral |
| 12 | RippleBand |
| 13 | Kamehemeha |
| 14 | Wave90 |
| 15 | Intersect |
| 16 | Shadow_disappear |
| 17 | Follow |
| 18 | SnakeUpDown |
| 19 | Customize |

**Đã xác minh:** cấu trúc từ mã gốc; so sánh byte với ID 0, 1, 2, 18. Chưa quan sát trực tiếp từng hiệu ứng firmware trên phần cứng.

## Màu tĩnh tùy chỉnh

Gửi report thiết lập hiệu ứng với ID 19, chờ 100 ms, sau đó gửi feature report 620 byte:

| Offset | Ý nghĩa |
|---:|---|
| 0 | `0x07` |
| 1 | `0x07` |
| 2–7 | 0 |
| `8 + 3*m` | R của vị trí ma trận m |
| `9 + 3*m` | G |
| `10 + 3*m` | B |

Đợi 50 ms sau khi gửi. App đã nhân độ sáng phần mềm vào các giá trị RGB, nên đặt độ sáng firmware 4 để tránh nhân độ sáng hai lần.

**Đã xác minh:** khớp từng byte với `SendCustomLight2Device` gốc; bản EXE gửi thành công cả hai report 620 byte trên thiết bị. Chưa kiểm tra giữ màu qua ngắt nguồn. Không có lệnh ghi flash bổ sung trong nhánh màu tùy chỉnh đã phân tích.

Thử `GetFeatureReport(7,620)` không trả lại dữ liệu cấu hình hữu ích (kết quả rỗng). Vì vậy app không tuyên bố đọc/khôi phục được thiết kế RGB trước đó; preset lưu trong app là thiết kế của app.

## Phạm vi

App chỉ mở đúng USB ID và ba usage collection nêu trên. Không gửi lệnh cập nhật firmware, factory reset, remap, macro hoặc debounce. Nhánh mã gốc có điều kiện loại một trạng thái kết nối khỏi RGB trực tiếp; bản này chủ động giới hạn ở kết nối USB đã kiểm tra. Chưa kiểm chứng receiver/Bluetooth.

Renderer chạy với `nodeIntegration: false`, `contextIsolation: true`, `sandbox: true`. IPC chỉ cho phép các thao tác cấp ứng dụng; không cung cấp API ghi HID tùy ý. Giá trị màu và preset được kiểm tra ở main process.

Tài liệu thư viện: [node-hid](https://github.com/node-hid/node-hid), [Electron context isolation](https://www.electronjs.org/docs/latest/tutorial/context-isolation), [Electron security](https://www.electronjs.org/docs/latest/tutorial/security).
