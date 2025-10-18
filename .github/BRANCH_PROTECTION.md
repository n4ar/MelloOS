# Branch Protection Setup Guide

## การตั้งค่า Branch Protection สำหรับ Develop Branch

เพื่อให้แน่ใจว่าทุก commit ที่เข้า develop branch ผ่านการทดสอบก่อน และต้องผ่าน Pull Request เท่านั้น ให้ทำตามขั้นตอนดังนี้:

### ขั้นตอนการตั้งค่าใน GitHub

1. **ไปที่ Repository Settings**
   - เปิด repository ของคุณบน GitHub
   - คลิกที่ `Settings` (ด้านบนขวา)

2. **เข้าสู่ Branch Protection Rules**
   - ในเมนูด้านซ้าย คลิก `Branches`
   - ในส่วน "Branch protection rules" คลิก `Add rule` หรือ `Add branch protection rule`

3. **ตั้งค่าสำหรับ Develop Branch**
   
   **Branch name pattern:** `develop`
   
   **เปิดใช้งานตัวเลือกต่อไปนี้:**
   
   ✅ **Require a pull request before merging**
   - ✅ Require approvals (แนะนำ: 1 approval)
   - ✅ Dismiss stale pull request approvals when new commits are pushed
   
   ✅ **Require status checks to pass before merging**
   - ✅ Require branches to be up to date before merging
   - เลือก status check: `test` (จาก workflow test-develop.yml)
   
   ✅ **Require conversation resolution before merging**
   
   ✅ **Do not allow bypassing the above settings**
   
   ⚠️ **ตัวเลือกเพิ่มเติม (แนะนำ):**
   - ✅ Require linear history (ป้องกัน merge commits)
   - ✅ Include administrators (ใช้กฎกับ admin ด้วย)

4. **บันทึกการตั้งค่า**
   - คลิก `Create` หรือ `Save changes`

### ขั้นตอนการตั้งค่าสำหรับ Main Branch (Production)

ทำซ้ำขั้นตอนเดียวกันสำหรับ `main` branch:

**Branch name pattern:** `main`

**เปิดใช้งานตัวเลือกเดียวกัน** แต่เพิ่ม:
- ✅ Require approvals: 2 approvals (เพิ่มความปลอดภัย)
- ✅ Restrict who can push to matching branches (เฉพาะ maintainers)

---

## Workflow ที่ถูกสร้างขึ้น

### 1. `test-develop.yml` - การทดสอบอัตโนมัติ

Workflow นี้จะทำงานเมื่อ:
- มีการ push ไปยัง `develop` branch
- มีการสร้าง Pull Request ไปยัง `develop` หรือ `main` branch

**ขั้นตอนการทดสอบ:**
1. ✅ Build kernel
2. ✅ ตรวจสอบ build artifacts (verify_build.sh)
3. ✅ สร้าง ISO image
4. ✅ ทดสอบการ boot ใน QEMU (test_boot.sh)

### 2. `build-and-release.yml` - การ Release (เดิม)

Workflow นี้ยังคงทำงานเมื่อ:
- มีการสร้าง tag เวอร์ชัน (เช่น v1.0.0)
- Manual trigger

---

## การใช้งาน

### สำหรับ Developer

**ห้าม push โดยตรงไปยัง develop:**
```bash
# ❌ ห้ามทำแบบนี้
git checkout develop
git commit -m "some changes"
git push origin develop
```

**ต้องสร้าง Pull Request:**
```bash
# ✅ ทำแบบนี้แทน
git checkout -b feature/my-feature
git commit -m "Add new feature"
git push origin feature/my-feature
# จากนั้นสร้าง Pull Request บน GitHub
```

### ขั้นตอนการทำงาน

1. **สร้าง Feature Branch**
   ```bash
   git checkout develop
   git pull origin develop
   git checkout -b feature/memory-allocator
   ```

2. **พัฒนาและ Commit**
   ```bash
   git add .
   git commit -m "Implement memory allocator"
   git push origin feature/memory-allocator
   ```

3. **สร้าง Pull Request**
   - ไปที่ GitHub repository
   - คลิก "Compare & pull request"
   - เลือก base branch: `develop`
   - เขียน description อธิบายการเปลี่ยนแปลง
   - คลิก "Create pull request"

4. **รอการทดสอบอัตโนมัติ**
   - GitHub Actions จะรัน test อัตโนมัติ
   - ถ้าผ่านทุก test จะเห็น ✅ สีเขียว
   - ถ้าไม่ผ่าน จะเห็น ❌ สีแดง พร้อมรายละเอียด error

5. **Review และ Merge**
   - รอให้ reviewer approve (ถ้าตั้งค่าไว้)
   - แก้ไข comments (ถ้ามี)
   - เมื่อทุกอย่างผ่านแล้ว คลิก "Merge pull request"

---

## การตรวจสอบ Status

### ดู Workflow Runs
1. ไปที่ tab `Actions` บน GitHub
2. เลือก workflow "Test on Develop Branch"
3. ดูรายละเอียดการรันแต่ละครั้ง

### ดู Branch Protection Status
1. ไปที่ `Settings` > `Branches`
2. ดูกฎที่ตั้งไว้สำหรับแต่ละ branch

---

## Troubleshooting

### ถ้า Test ไม่ผ่าน

1. **ดู logs ใน GitHub Actions**
   - คลิกที่ ❌ ใน Pull Request
   - ดูรายละเอียด error ในแต่ละ step

2. **ทดสอบ locally ก่อน push**
   ```bash
   # Build และทดสอบ
   make build
   ./tools/verify_build.sh
   make iso
   ./tools/test_boot.sh
   ```

3. **แก้ไขและ push ใหม่**
   ```bash
   git add .
   git commit -m "Fix test failures"
   git push origin feature/my-feature
   ```
   - GitHub Actions จะรันอัตโนมัติอีกครั้ง

### ถ้าต้องการ bypass (ฉุกเฉิน)

ถ้าคุณเป็น admin และต้องการ bypass ในกรณีฉุกเฉิน:
1. ไปที่ Pull Request
2. คลิก "Merge without waiting for requirements to be met"
3. ⚠️ ใช้เฉพาะกรณีจำเป็นจริงๆ เท่านั้น

---

## Best Practices

1. ✅ รัน test locally ก่อน push
2. ✅ เขียน commit message ที่ชัดเจน
3. ✅ แยก feature เป็น branch ย่อยๆ
4. ✅ Review code ของตัวเองก่อนขอ review
5. ✅ ตอบ comments และแก้ไขตาม feedback
6. ✅ Keep branches up-to-date กับ develop
7. ✅ Delete branch หลัง merge แล้ว

---

## สรุป

หลังจากตั้งค่า Branch Protection แล้ว:

- ✅ ทุก commit ต้องผ่าน Pull Request
- ✅ ทุก PR ต้องผ่าน automated tests
- ✅ ทุก PR ต้องได้รับ approval (ถ้าตั้งค่าไว้)
- ✅ ป้องกันการ push โดยตรงไปยัง develop/main
- ✅ รับประกันคุณภาพของ code ใน develop branch

การตั้งค่านี้จะช่วยให้ทีมทำงานร่วมกันได้อย่างมีประสิทธิภาพและปลอดภัยมากขึ้น! 🚀
