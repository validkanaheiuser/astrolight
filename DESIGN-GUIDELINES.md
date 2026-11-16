# Astrolight 3.2 — cơ sở thiết kế và quyết định

Ngày nghiên cứu: 14/11/2026. Đây là bản triển khai dựa trên tài liệu do các tác giả/tổ chức bên dưới xuất bản; không phải thiết kế được họ thẩm định. Chưa có nghiên cứu người dùng độc lập.

## Nguồn và cách áp dụng

| Nguồn | Nguyên tắc áp dụng | Thay đổi trong Astrolight |
|---|---|---|
| [Jakob Nielsen và NN/g: 10 usability heuristics](https://www.nngroup.com/articles/ten-usability-heuristics/) | Trạng thái rõ; nhận biết thay vì ghi nhớ; quyền hoàn tác | Phân biệt bản nháp đã lưu, chưa lưu và lần ghi thiết bị; chỉ rõ phím/layer đang sửa; Undo/Redo |
| [Microsoft: Content layout and spacing](https://learn.microsoft.com/en-us/windows/apps/design/basics/content-basics) | Khoảng cách nhất quán, thứ bậc chữ, nhóm điều khiển liên quan | Nhịp 4/8 px, nhóm cách nhau 24 px; heading 26, section 16, body 14, caption 12 px |
| [Microsoft: Commanding basics](https://learn.microsoft.com/en-us/windows/apps/design/basics/commanding-basics) | Lệnh gần nội dung, phản hồi ngay; hoàn tác thao tác có thể đảo ngược | Inspector cạnh bàn phím; footer ghi/lưu; reset layer có Undo; review trước ghi thiết bị |
| [W3C WAI: Contrast minimum](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html) | Chữ thông thường đạt 4,5:1; chữ lớn 3:1 | Kiểm tra màu chữ chính/phụ trên bề mặt tối; không chỉ dùng màu để báo phím đã gán |
| [W3C WAI: Target size minimum](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html) | Vùng bấm tối thiểu 24×24 CSS px hoặc khoảng cách tương đương | Nút thường 36–40 px; remap giữ bàn phím đủ rộng và cuộn trong khung ở màn nhỏ; dropdown vị trí là cách chọn thay thế |
| [VIA source: keycode pane](https://github.com/the-via/app/blob/main/src/components/panes/configure-panes/keycode.tsx) | Tham khảo mô hình chọn vị trí/layer và thư viện chức năng | Giữ tương tác trực tiếp trên sơ đồ, nhóm chức năng có tên; triển khai riêng cho giao thức Lucky65 |

Không tuyên bố đạt đầy đủ WCAG chỉ từ các kiểm tra này. Không sử dụng firmware QMK/VIA, không thay giao thức HID.

## Định hướng hình ảnh

Không gian chỉnh bàn phím, với bàn phím là đối tượng trung tâm. Logo có loop nhẹ theo yêu cầu; giao diện còn lại yên tĩnh, chỉ chuyển động khi người dùng thao tác. Nền charcoal, keycap graphite, màu lilac của logo dùng cho lựa chọn, màu xanh ngọc dùng cho trạng thái kết nối.

- Nền: #131419; bề mặt #1c1e26; bề mặt nổi #252834.
- Chữ chính #f2f2f7; chữ phụ #b2b4c4; accent #c1afff. Success #84d6c0; cảnh báo #f1ca8d.
- Font Segoe UI Variable / Segoe UI: native Windows, hỗ trợ tiếng Việt, không tải font mạng. Heading 26/32; body 14/21; caption 12/18.
- Sidebar 176 px; thu gọn khi cửa sổ hẹp. Nội dung căn trái; vùng làm việc hai cột, inspector 272 px; xếp một cột dưới 1120 px.
- Bán kính theo vai trò: nút 8, keycap 6, vùng thiết bị 16. Tránh chia mọi thứ thành card giống nhau.

## Wireframe và tự phản biện trước khi code

Phương án A: bàn phím trên cùng, mọi điều khiển bên dưới. Dễ làm nhưng phải cuộn qua thư viện để đổi màu; chọn phím xong khó thấy mình đang sửa gì. Loại.

Phương án B được chọn:

    Điều hướng | Thiết bị / trạng thái             Kết nối
               | Tên trang / hướng dẫn ngắn
               | Bàn phím + layer          | Inspector
               | Thư viện hiệu ứng/phím    | Phím đã chọn
               |                          | Thông tin / tổ hợp
               | Bản nháp / Undo / Redo    Lưu / Xem trước ghi

Điều chỉnh sau tự phản biện: bỏ slogan lớn chiếm chỗ; bỏ đèn RGB chuyển động ở màn remap vì làm nhiễu chữ; giữ phím đích, binding và layer ở cùng ngữ cảnh; dùng keycap thật làm điểm nhấn thay vì trang trí gradient.

## Yêu cầu tương tác

- Chọn một vị trí trên bàn phím hoặc dropdown; đổi layer vẫn giữ vị trí đó.
- Phím đã gán có nhãn chức năng và nhãn vị trí gốc. Tổ hợp modifier được đồng bộ lại khi chọn phím khác để không vô tình giữ Ctrl/Alt.
- Tìm kiếm xuyên nhóm, hỗ trợ tên/alias tiếng Việt không dấu. Có trạng thái không có kết quả và macro trống có lối đi tạo macro.
- Bản nháp có Undo/Redo; lưu máy không đồng nghĩa ghi thiết bị. Không giả vờ đã đọc keymap firmware.
- Không gửi remap khi chọn keycode: chỉ gửi sau màn review có nút ghi rõ ràng.
- Tab/Enter/Space chọn phím; mũi tên di chuyển trên sơ đồ khi có focus; focus rõ; tôn trọng Reduced Motion.
- Giữ nguyên engine 3.1: 30 FPS và ưu tiên gõ bật mặc định.
