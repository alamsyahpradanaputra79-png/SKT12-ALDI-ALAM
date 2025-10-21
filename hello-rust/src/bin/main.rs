#![no_std]
#![no_main]

use panic_halt as _;
use esp_hal::{Config, uart::{Uart, Config as UartConfig}};
use esp_println::{println};
use esp_hal::time::{Instant, Duration};

esp_bootloader_esp_idf::esp_app_desc!();

const BAUD: u32 = 9_600;
const SID:  u8  = 1;

// ~5 ms jeda turn-around @9600 (kasar; sesuaikan bila perlu)
const TURNAROUND_SPINS: u32 = 50_000;
// timeout polling RX (kasar)
const TIMEOUT_SPINS:    u32 = 200_000;
// taksiran spin untuk ~1 ms (tuning sesuai board/clock)

#[esp_hal::main]
fn main() -> ! {
    let p = esp_hal::init(Config::default());

    // UART1: TX=GPIO17, RX=GPIO18 (ubah sesuai wiring)
    let mut uart = Uart::new(p.UART1, UartConfig::default().with_baudrate(BAUD))
        .expect("UART1 init failed")
        .with_tx(p.GPIO17)
        .with_rx(p.GPIO18);

    println!("\n=== SHT20 (RS485) loop: FC=0x04, qty=1 ===");

    loop {
        // ---------- BACA RH @ 0x0001 ----------
        // Request: [ID][FC][ADDR_H][ADDR_L][CNT_H][CNT_L][CRC_L][CRC_H]
        let mut req = [0u8; 8];
        req[0] = SID;
        req[1] = 0x04;
        req[2..4].copy_from_slice(&0x0002u16.to_be_bytes());
        req[4..6].copy_from_slice(&1u16.to_be_bytes());
        let crc = crc16(&req[..6]);
        req[6] = (crc & 0xFF) as u8;      // CRC Lo
        req[7] = (crc >> 8) as u8;        // CRC Hi

        // print_hex("TX", &req);
        let _ = uart.write(&req);
        let _ = uart.flush();
        short_spin(TURNAROUND_SPINS);

        // Baca minimal 7 byte (ID+FC+BC+DATA2+CRC2), polling per 1 byte
        let mut rx = [0u8; 32];
        let mut n = 0usize;
        let mut spins = 0u32;
        while spins < TIMEOUT_SPINS && n < rx.len() {
            let mut b = [0u8; 1];
            match uart.read(&mut b) {
                Ok(1) => { rx[n] = b[0]; n += 1; if n >= 7 { break; } }
                _ => { short_spin(1_000); spins += 1; }
            }
        }
        // if n > 0 { print_hex("RX", &rx[..n]); }
        if n >= 7 && (rx[1] & 0x80) == 0 && rx[2] == 2 && check_crc(&rx[..n]) {
            let raw_rh = u16::from_be_bytes([rx[3], rx[4]]);
            println!("RH = {:.1} %", raw_rh as f32 / 10.0);
        } else {
            println!("No/invalid reply for 0x0001 (RH)");
        }

        // ---------- BACA T @ 0x0002 ----------
        req[2..4].copy_from_slice(&0x0001u16.to_be_bytes());
        let crc2 = crc16(&req[..6]);
        req[6] = (crc2 & 0xFF) as u8;
        req[7] = (crc2 >> 8) as u8;

        // print_hex("TX", &req);
        let _ = uart.write(&req);
        let _ = uart.flush();
        short_spin(TURNAROUND_SPINS);

        n = 0;
        spins = 0;
        while spins < TIMEOUT_SPINS && n < rx.len() {
            let mut b = [0u8; 1];
            match uart.read(&mut b) {
                Ok(1) => { rx[n] = b[0]; n += 1; if n >= 7 { break; } }
                _ => { short_spin(1_000); spins += 1; }
            }
        }
        // if n > 0 { print_hex("RX", &rx[..n]); }
        if n >= 7 && (rx[1] & 0x80) == 0 && rx[2] == 2 && check_crc(&rx[..n]) {
            let raw_t = u16::from_be_bytes([rx[3], rx[4]]);
            println!("T  = {:.1} °C", raw_t as f32 / 10.0);
        } else {
            println!("No/invalid reply for 0x0002 (Temp)");
        }

        sleep(Duration::from_millis(1000));
    }
}

// ===== Utils (tanpa nyebut Blocking/DriverMode) =====
#[inline(always)]
fn sleep(dur: Duration) {
    let start = Instant::now();
    // tunggu sampai waktu berjalan ≥ dur
    while start.elapsed() < dur {
        core::hint::spin_loop();
    }
}

fn short_spin(iter: u32) { for _ in 0..iter { core::hint::spin_loop(); } }

fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &b in data {
        crc ^= b as u16;
        for _ in 0..8 {
            crc = if (crc & 1) != 0 { (crc >> 1) ^ 0xA001 } else { crc >> 1 };
        }
    }
    crc
}
fn check_crc(frame: &[u8]) -> bool {
    if frame.len() < 3 { return false; }
    let calc = crc16(&frame[..frame.len() - 2]);
    frame[frame.len() - 2] == (calc & 0xFF) as u8 && frame[frame.len() - 1] == (calc >> 8) as u8
}
