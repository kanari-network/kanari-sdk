# การใช้งาน kari move publish

คำสั่ง `kari move publish` ใช้สำหรับอัปโหลดโมดูล Move ขึ้นไปยัง Mona VM blockchain

## รูปแบบคำสั่ง

### พารามิเตอร์

- `MODULE_PATH`: พาธไปยังโมดูล Move ที่ต้องการอัปโหลด (ค่าเริ่มต้นคือไดเร็กทอรีปัจจุบัน)
- `--gas-budget=N`: จำนวน gas units ที่จะใช้ (ค่าเริ่มต้นคือ 3,000,000)
- `--skip-verify`: ข้ามขั้นตอนการตรวจสอบโมดูล
- `--address=ADDRESS`: แอดเดรสที่จะใช้อัปโหลดโมดูล (ถ้าไม่ระบุจะใช้แอดเดรสจากวอลเล็ตหรือใช้ค่าเริ่มต้น 0x1)

### ตัวอย่าง

```bash
# อัปโหลดโมดูลจากไดเร็กทอรีปัจจุบัน
kari move publish

# อัปโหลดโมดูลจากพาธที่กำหนด
kari move publish /path/to/module

# กำหนดจำนวน gas units
kari move publish --gas-budget=5000000

# อัปโหลดไปยังแอดเดรสเฉพาะ
kari move publish --address=0x123abc

# ข้ามการตรวจสอบโมดูล
kari move publish --skip-verify
```

# การใช้งาน kari move call

คำสั่ง `kari move call` ใช้สำหรับเรียกใช้ฟังก์ชันในโมดูล Move ที่ถูกอัปโหลดไปยัง blockchain แล้ว

## รูปแบบคำสั่ง