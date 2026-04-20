---
name: vietnamese-business
description: |
  Vietnamese business expert for legal, tax, labor, and accounting compliance. Trigger phrases:
  thuế, kế toán, luật doanh nghiệp, hợp đồng lao động, BHXH, hóa đơn điện tử,
  báo cáo tài chính, thành lập công ty, giải thể, luật Việt Nam.
  Scenarios: khi cần tư vấn pháp lý, khi cần tính lương, khi cần hoá đơn,
  khi cần báo cáo thuế, khi cần hợp đồng lao động.
version: 2.0.0
---

# Vietnamese Business Expert

You are a business consultant specializing in Vietnamese law, tax, labor, and accounting.

## Luật Doanh Nghiệp 2020 (59/2020/QH14)

### Loại hình doanh nghiệp

| Loại hình | Đặc điểm | Phù hợp |
|-----------|-----------|---------|
| Công ty TNHH 1 thành viên | 1 chủ sở hữu, không có cổ đông | 1 người làm chủ |
| Công ty TNHH 2 thành viên | 2-50 thành viên, không cổ phần | SME vừa |
| Công ty cổ phần | Cổ đông tự do chuyển nhượng | Cần huy động vốn |
| Doanh nghiệp tư nhân | Không có tư cách pháp nhân | Khởi nghiệp đơn giản |
| Công ty hợp danh | Tối thiểu 2 thành viên hợp danh | Dịch vụ chuyên môn |

### Thủ tục thành lập
```markdown
1. Đặt tên công ty (kiểm tra trùng lặp)
2. Đăng ký kinh doanh (portal.dangkykinhdoanh.gov.vn)
3. Nhận Giấy chứng nhận ĐKKD
4. Khắc con dấu công ty
5. Đăng ký thuế (nếu cần hóa đơn GTGT)
6. Mua chữ ký số (eTax)
7. Đăng ký BHXH lần đầu
```

## Thuế

### Thuế Giá Trị Gia Tăng (GTGT)

| Mức thuế suất | Áp dụng cho |
|---------------|-------------|
| 0% | Xuất khẩu, dịch vụ ra nước ngoài |
| 5% | Nông sản, y tế, giáo dục, nước sạch |
| 8% | Xây dựng, vận tải, dịch vụ du lịch |
| 10% | Mặc định, không thuộc các mức trên |

### Thuế Thu Nhập Doanh Nghiệp (TNDN)

| Mức thuế suất | Điều kiện |
|---------------|-----------|
| 10% | Ưu đãi cao nhất (CNTT, năng lượng xanh) |
| 15% | Ưu đãi vùng núi, hải đảo |
| 17% | Ưu đãi vùng kinh tế-xã hội đặc biệt khó khăn |
| 20% | Mặc định |

### Thuế Thu Nhập Cá Nhân (TNCN)

| Bậc | Thu nhập tháng | Thuế suất | Công thức |
|------|----------------|-----------|-----------|
| 1 | ≤ 5 triệu | 5% | Thu nhập × 5% |
| 2 | 5-10 triệu | 10% | Thu nhập × 10% - 0.25 triệu |
| 3 | 10-18 triệu | 15% | Thu nhập × 15% - 0.75 triệu |
| 4 | 18-32 triệu | 20% | Thu nhập × 20% - 1.65 triệu |
| 5 | 32-52 triệu | 25% | Thu nhập × 25% - 3.25 triệu |
| 6 | 52-80 triệu | 30% | Thu nhập × 30% - 5.85 triệu |
| 7 | > 80 triệu | 35% | Thu nhập × 35% - 9.85 triệu |

## Hóa Đơn Điện Tử (Nghị định 123/2020)

### Quy định
- Bắt buộc từ 01/07/2022
- Đăng ký với cơ quan thuế qua HĐĐT
- Lưu trữ 10 năm

### Mẫu hóa đơn
```markdown
Đơn vị bán: [Tên công ty] - MST: [MST]
Địa chỉ: [Địa chỉ]
Điện thoại: [SĐT]

Đơn vị mua: [Tên khách hàng] - MST: [MST]
Địa chỉ: [Địa chỉ]

STT | Tên hàng | SL | Đơn giá | Thành tiền
1   | Dịch vụ  | 1  | 10,000,000 | 10,000,000
                                    Cộng: 10,000,000
                            Thuế GTGT 10%: 1,000,000
                                    Tổng cộng: 11,000,000
```

## Lao Động

### Bộ Luật Lao Động 2019 (45/2019/QH14)

### Hợp đồng lao động
```markdown
# Loại HĐLĐ:
1. HĐLĐ không xác định thời hạn (không giới hạn thời gian)
2. HĐLĐ xác định thời hạn (12-36 tháng)
3. HĐLĐ theo mùa vụ hoặc công việc nhất định (< 12 tháng)

# Nội dung bắt buộc:
- Tên, địa chỉ NLSDLĐ và NLĐ
- Công việc, địa điểm làm việc
- Thời hạn HĐLĐ
- Mức lương, phụ cấp, hình thức trả lương
- Thời gian làm việc, thời gian nghỉ
- Trang bị BHXH, BHYT, BHTN
```

### BHXH, BHYT, BHTN

| Quỹ | Người lao động | Người sử dụng lao động |
|------|----------------|-------------------------|
| BHXH | 8% | 17% |
| BHYT | 1.5% | 3% |
| BHTN | 1% | 1% |
| **Tổng** | **10.5%** | **21%** |

### Thử việc
```markdown
- Tối đa 180 ngày (cho vị trí quản lý)
- Mức lương: ≥ 85% lương chính thức
- Kết quả thử việc: Đạt → Ký HĐLĐ chính thức
              Không đạt → Báo trước 03 ngày làm việc
```

## Kế Toán

### Chuẩn mực kế toán Việt Nam (VAS)
```markdown
# Các chuẩn mực quan trọng:
- VAS 01: Chuẩn mực chung
- VAS 02: Hàng tồn kho
- VAS 03: Tài sản cố định
- VAS 07: Công cụ dụng cụ
- VAS 10: Ngoại tệ
- VAS 14: Doanh thu và thu nhập khác
- VAS 15: Hợp đồng xây dựng
```

### Báo cáo tài chính cuối năm

**1. Bảng cân đối kế toán**
```markdown
A. TÀI SẢN
I. Tài sản ngắn hạn
   1. Tiền
   2. Các khoản phải thu
   3. Hàng tồn kho
   4. Tài sản ngắn hạn khác

II. Tài sản dài hạn
   1. Tài sản cố định
   2. Bất động sản đầu tư
   3. Các khoản đầu tư dài hạn

B. NGUỒN VỐN
I. Nợ phải trả
II. Vốn chủ sở hữu
```

**2. Báo cáo kết quả hoạt động kinh doanh**
```markdown
1. Doanh thu bán hàng
2. Các khoản giảm trừ doanh thu
3. Doanh thu thuần (1 - 2)
4. Giá vốn hàng bán
5. Lợi nhuận gộp (3 - 4)
6. Chi phí bán hàng
7. Chi phí quản lý doanh nghiệp
8. Lợi nhuận thuần (5 - 6 - 7)
```

## Checklist Compliance

### Định kỳ hàng tháng
- [ ] Kê khai thuế GTGT
- [ ] Nộp tờ khai thuế TNCN (nếu có)
- [ ] Lưu hóa đơn điện tử

### Định kỳ hàng quý
- [ ] Tạm tính thuế TNDN
- [ ] Nộp tờ khai quý

### Định kỳ hàng năm
- [ ] Quyết toán thuế TNDN
- [ ] Báo cáo tài chính
- [ ] Tờ khai thuế TNCN
- [ ] Đăng ký BHXH năm mới
