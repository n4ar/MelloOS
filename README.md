# MelloOS

MelloOS เป็นระบบปฏิบัติการที่สร้างขึ้นตั้งแต่ศูนย์ด้วยภาษา Rust โดยมุ่งเน้นความปลอดภัย ความทันสมัย และความสามารถในการขยายต่อได้ในอนาคต โปรเจกต์นี้เริ่มต้นจากการพัฒนา Kernel ที่สามารถบูตผ่าน UEFI bootloader และแสดงข้อความบนหน้าจอได้

## Features

- ✨ Bare-metal kernel เขียนด้วย Rust (`no_std`)
- 🚀 บูตผ่าน UEFI firmware ด้วย Limine bootloader
- 🖥️ Framebuffer driver สำหรับการแสดงผลบนหน้าจอ
- 🔧 Build system อัตโนมัติด้วย Makefile
- 🧪 ทดสอบได้ง่ายด้วย QEMU emulator

## Prerequisites

ก่อนเริ่มต้น คุณต้องติดตั้ง dependencies ต่อไปนี้:

### Required Dependencies

1. **Rust Toolchain**
   - Rust compiler และ Cargo package manager
   - Target สำหรับ bare-metal x86_64

2. **QEMU**
   - QEMU system emulator สำหรับ x86_64

3. **xorriso**
   - ISO image creation tool

4. **OVMF**
   - UEFI firmware สำหรับ QEMU

5. **Git**
   - สำหรับดาวน์โหลด Limine bootloader

## Installation

### macOS

```bash
# ติดตั้ง Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# เพิ่ม Rust target สำหรับ bare-metal
rustup target add x86_64-unknown-none

# ติดตั้ง QEMU, xorriso ด้วย Homebrew
brew install qemu xorriso

# ติดตั้ง OVMF (UEFI firmware)
brew install --cask edk2-ovmf
```

### Linux (Ubuntu/Debian)

```bash
# ติดตั้ง Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# เพิ่ม Rust target สำหรับ bare-metal
rustup target add x86_64-unknown-none

# ติดตั้ง QEMU, xorriso, OVMF
sudo apt update
sudo apt install qemu-system-x86 xorriso ovmf git
```

### Linux (Arch)

```bash
# ติดตั้ง Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# เพิ่ม Rust target สำหรับ bare-metal
rustup target add x86_64-unknown-none

# ติดตั้ง QEMU, xorriso, OVMF
sudo pacman -S qemu-full xorriso edk2-ovmf git
```

## Building

### Build Kernel

คอมไพล์ kernel binary:

```bash
make build
```

คำสั่งนี้จะ:
- รัน `cargo build --release` เพื่อคอมไพล์ kernel
- สร้าง ELF binary ที่ `kernel/target/x86_64-unknown-none/release/kernel`

### Create Bootable ISO

สร้าง ISO image ที่สามารถบูตได้:

```bash
make iso
```

คำสั่งนี้จะ:
- คอมไพล์ kernel (ถ้ายังไม่ได้ build)
- ดาวน์โหลด Limine bootloader (ถ้ายังไม่มี)
- สร้างโครงสร้างไดเรกทอรีสำหรับ ISO
- คัดลอก kernel binary และ bootloader files
- สร้าง `mellos.iso` ด้วย xorriso
- ติดตั้ง Limine bootloader ลงใน ISO

### Run in QEMU

รัน kernel ใน QEMU emulator:

```bash
make run
```

คำสั่งนี้จะ:
- สร้าง ISO image (ถ้ายังไม่มี)
- เริ่มต้น QEMU ด้วย UEFI firmware
- บูต MelloOS จาก ISO
- แสดงหน้าต่าง QEMU พร้อมข้อความ "Hello from my kernel ✨"

หากต้องการปิด QEMU ให้กด `Ctrl+C` ใน terminal หรือปิดหน้าต่าง QEMU

### Clean Build Artifacts

ลบไฟล์ที่สร้างขึ้นจากการ build:

```bash
make clean
```

คำสั่งนี้จะลบ:
- Cargo build artifacts
- ISO image
- Temporary directories

## Project Structure

```
mellos/
├── .cargo/
│   └── config.toml          # Cargo build configuration
├── kernel/
│   ├── Cargo.toml           # Kernel dependencies
│   ├── linker.ld            # Linker script
│   └── src/
│       ├── main.rs          # Kernel entry point
│       ├── framebuffer.rs   # Framebuffer driver
│       └── panic.rs         # Panic handler
├── boot/
│   └── limine.cfg           # Bootloader configuration
├── tools/
│   └── qemu.sh              # QEMU launch script
├── Makefile                 # Build automation
└── README.md                # This file
```

## Troubleshooting

### Build Errors

**Problem:** `error: target 'x86_64-unknown-none' not found`

**Solution:** ติดตั้ง Rust target:
```bash
rustup target add x86_64-unknown-none
```

---

**Problem:** `cargo: command not found`

**Solution:** ติดตั้ง Rust toolchain และเพิ่ม Cargo ใน PATH:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

---

**Problem:** Linker errors เกี่ยวกับ `_start` symbol

**Solution:** ตรวจสอบว่า `linker.ld` ถูกกำหนดใน `.cargo/config.toml` และมี `#[no_mangle]` บน `_start` function

### ISO Creation Errors

**Problem:** `xorriso: command not found`

**Solution:** ติดตั้ง xorriso:
- macOS: `brew install xorriso`
- Ubuntu/Debian: `sudo apt install xorriso`
- Arch: `sudo pacman -S xorriso`

---

**Problem:** `limine: command not found` หรือ Limine files ไม่พบ

**Solution:** Makefile จะดาวน์โหลด Limine อัตโนมัติ แต่ถ้ามีปัญหา ให้ลอง clone manually:
```bash
git clone https://github.com/limine-bootloader/limine.git --branch=v8.x-binary --depth=1
cd limine
make
```

### QEMU Errors

**Problem:** `qemu-system-x86_64: command not found`

**Solution:** ติดตั้ง QEMU:
- macOS: `brew install qemu`
- Ubuntu/Debian: `sudo apt install qemu-system-x86`
- Arch: `sudo pacman -S qemu-full`

---

**Problem:** `Could not open '/usr/share/ovmf/OVMF.fd'`

**Solution:** OVMF firmware path อาจแตกต่างกันในแต่ละระบบ แก้ไข `tools/qemu.sh`:

- macOS (Homebrew): `/opt/homebrew/share/edk2-ovmf/x64/OVMF.fd`
- Ubuntu/Debian: `/usr/share/OVMF/OVMF_CODE.fd`
- Arch: `/usr/share/edk2-ovmf/x64/OVMF.fd`

หรือรัน QEMU โดยไม่ใช้ UEFI (legacy BIOS mode):
```bash
qemu-system-x86_64 -M q35 -m 2G -cdrom mellos.iso -boot d
```

---

**Problem:** QEMU เปิดแต่หน้าจอดำ

**Solution:** 
1. ตรวจสอบว่า ISO ถูกสร้างสำเร็จ: `ls -lh mellos.iso`
2. ตรวจสอบ serial output ใน terminal
3. ลอง rebuild: `make clean && make run`

---

**Problem:** ข้อความไม่แสดงบนหน้าจอ QEMU

**Solution:**
1. ตรวจสอบว่า framebuffer request ใน `main.rs` ถูกต้อง
2. ตรวจสอบว่า Limine configuration มี `PROTOCOL=limine`
3. ลอง rebuild kernel: `make clean && make build && make iso && make run`

### Runtime Errors

**Problem:** Kernel panic ทันทีหลังบูต

**Solution:**
1. ตรวจสอบ panic message ใน serial output
2. ตรวจสอบว่า framebuffer response จาก Limine ไม่เป็น null
3. เพิ่ม debug output ใน panic handler

---

**Problem:** Kernel หยุดทำงานโดยไม่แสดง error

**Solution:**
1. เพิ่ม serial port debugging
2. ใช้ QEMU monitor: กด `Ctrl+Alt+2` เพื่อเข้า monitor mode
3. ตรวจสอบ CPU state ด้วย `info registers` ใน QEMU monitor

## Testing

### Automated Build Verification

รันการทดสอบอัตโนมัติเพื่อตรวจสอบว่า build process ทำงานถูกต้อง:

```bash
./tools/verify_build.sh
```

สคริปต์นี้จะตรวจสอบ:
- ✓ Kernel binary ถูกสร้างสำเร็จ
- ✓ ข้อความที่ต้องการอยู่ใน kernel
- ✓ ISO image ถูกสร้างและมี kernel
- ✓ QEMU พร้อมใช้งาน
- ✓ Limine bootloader files ครบถ้วน
- ✓ Configuration files ถูกต้อง

### Manual Visual Testing

เนื่องจาก kernel แสดงผลผ่าน framebuffer (graphical display) คุณต้องทดสอบด้วยตาเองว่าข้อความแสดงถูกต้อง:

```bash
make run
```

**คาดหวังผลลัพธ์:**
1. หน้าต่าง QEMU เปิดขึ้น
2. Limine bootloader menu ปรากฏ (รอ 3 วินาที)
3. Kernel บูตอัตโนมัติ
4. ข้อความ **"Hello from my kernel ✨"** แสดงบนหน้าจอ
5. Kernel อยู่ในสถานะรอ (infinite loop)

### Test Results

ดูผลการทดสอบโดยละเอียดได้ที่ `TEST_RESULTS.md`

## Development

### Adding New Features

1. แก้ไขโค้ดใน `kernel/src/`
2. Build และทดสอบ: `make run`
3. ตรวจสอบผลลัพธ์ใน QEMU

### Debugging Tips

- ใช้ `serial stdio` ใน QEMU เพื่อดู debug output
- เพิ่ม `println!` macros (ต้อง implement serial driver ก่อน)
- ใช้ QEMU monitor สำหรับ low-level debugging
- ตรวจสอบ memory layout ด้วย `objdump -h kernel/target/x86_64-unknown-none/release/kernel`
- รัน automated tests ด้วย `./tools/verify_build.sh` ก่อนทดสอบใน QEMU

## Resources

- [Rust Embedded Book](https://rust-embedded.github.io/book/)
- [OSDev Wiki](https://wiki.osdev.org/)
- [Limine Bootloader](https://github.com/limine-bootloader/limine)
- [Writing an OS in Rust](https://os.phil-opp.com/)

## License

This project is open source and available for educational purposes.

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## Acknowledgments

- Limine bootloader team
- Rust embedded community
- OSDev community
