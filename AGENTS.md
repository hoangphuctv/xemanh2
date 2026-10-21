# AGENTS.md

## Project

XemAnh2 là trình xem ảnh desktop nhẹ, đa nền tảng, viết bằng Rust.

Mục tiêu chính:
- Nhẹ và khởi động nhanh.
- Phản hồi tốt khi xem và chuyển ảnh.
- Giao diện native, đơn giản.
- Code dễ hiểu, dễ bảo trì.
- Hạn chế dependency không cần thiết.

## Architecture

Công nghệ chính:
- `winit 0.30`: window, event loop, keyboard và mouse input.
- `softbuffer 0.4`: CPU framebuffer.
- `image 0.25`: decode và xử lý ảnh cơ bản.
- `kamadak-exif 0.6`: đọc EXIF và orientation.

Không sử dụng game engine.

Không thêm OpenGL, miniquad, macroquad, Vulkan, DirectX hoặc Metal nếu chưa có lý do kỹ thuật rõ ràng và được yêu cầu.

Ứng dụng không chạy game loop liên tục. Chỉ render lại khi trạng thái hiển thị thay đổi.

## Development Rules

Ưu tiên code đơn giản và trực tiếp.

Không over-engineering.

Mỗi thay đổi nên tập trung vào một tính năng hoặc một bug cụ thể.

Sau khi sửa code:
1. `cargo check`
2. `cargo build`
3. Chạy ứng dụng với `-v` để kiểm tra runtime khi thay đổi có ảnh hưởng đến behavior.
4. `git diff --check`
5. Commit với message ngắn, mô tả đúng thay đổi.

Khi debug runtime, không chỉ dựa vào `cargo check`. Phải chạy ứng dụng và kiểm tra log nếu có thể.

## Current Controls

- `Right`, `Down`, `PageDown`: ảnh tiếp theo.
- `Left`, `Up`, `PageUp`: ảnh trước.
- `Home`: ảnh đầu tiên.
- `End`: ảnh cuối cùng.
- `+` / `=` / Numpad `+`: zoom in.
- `-` / Numpad `-`: zoom out.
- `0` / Numpad `0`: reset zoom về fit window.
- Mouse wheel: zoom.
- `Space`: fullscreen.
- `Escape`: thoát fullscreen hoặc thoát ứng dụng.

Khi thêm shortcut mới, cập nhật README nếu đó là chức năng người dùng cần biết.

## Rendering

Ảnh được scale theo kích thước cửa sổ và `zoom`.

`zoom = 1.0` tương ứng với trạng thái fit-to-window.

Khi ảnh lớn hơn viewport, renderer phải clipping an toàn. Không được dùng phép toán unsigned có thể gây underflow khi offset âm.

Không để ảnh zoom lớn gây panic.

## Git

Branch hiện tại là `master`.

Commit nhỏ, có ý nghĩa và không gộp nhiều thay đổi không liên quan vào cùng một commit.

Không commit build artifacts hoặc file tạm.

## Future Direction

Các tính năng có thể phát triển tiếp:
- Pan ảnh khi zoom.
- EXIF orientation.
- Thông tin ảnh.
- Xử lý rotate.
- Copy/paste và thao tác file.
- Hiển thị gallery/thumbnail nếu cần.

Chỉ triển khai khi có yêu cầu hoặc khi cần thiết cho kiến trúc hiện tại.
