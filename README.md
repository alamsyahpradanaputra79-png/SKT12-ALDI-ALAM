# Implementasi Sistem Kontrol Terdistribusi Berbasis Embedded Rust pada ESP32-S3 dengan Integrasi DWSIM dan Cloud ThingsBoard

⚙️ Implementasi Sistem Kontrol Terdistribusi Berbasis Embedded Rust pada ESP32-S3 dengan Integrasi DWSIM dan Cloud ThingsBoard

Proyek ini merupakan hasil dari implementasi **Sistem Kontrol Terdistribusi** menggunakan **mikrokontroler ESP32-S3** yang diprogram dengan **Embedded Rust**.  
Sistem ini dirancang untuk melakukan pemantauan dan kendali proses industri secara real-time, dengan integrasi terhadap **simulator proses DWSIM** dan platform **IoT Cloud ThingsBoard** sebagai media visualisasi serta analisis data.

---

## 🎯 Tujuan Proyek
- Mengimplementasikan konsep **kontrol terdistribusi (Distributed Control System)** pada perangkat embedded berbasis Rust.  
- Mengintegrasikan simulasi proses dari **DWSIM** dengan sistem kontrol fisik melalui protokol TCP.  
- Mengirimkan dan memvisualisasikan data proses ke **Cloud ThingsBoard**.  
- Menunjukkan kemampuan Rust dalam sistem real-time embedded yang efisien dan andal.

---

## 🧩 Fitur Utama
- 📡 Komunikasi data dua arah antara ESP32-S3 dan DWSIM melalui TCP Socket.  
- 🧠 Pengendalian parameter proses (seperti suhu, tekanan, aliran, dsb.) secara otomatis dari hasil simulasi.  
- ☁️ Pengiriman data ke **ThingsBoard** untuk visualisasi cloud dashboard.  
- 🧮 Pemantauan kondisi sistem dan logging data secara periodik.  
- 🔒 Arsitektur terdistribusi yang memungkinkan ekspansi ke beberapa node kontrol.

---

## ⚙️ Arsitektur Sistem

```

[ DWSIM Simulator ]
│ (TCP)
▼
[ ESP32-S3 + Embedded Rust ]
│ (MQTT/HTTP)
▼
[ ThingsBoard Cloud Dashboard ]

```

**Penjelasan alur data:**
1. DWSIM menghasilkan data proses (misal suhu, tekanan).  
2. ESP32-S3 menerima data tersebut via TCP, lalu melakukan pengendalian sesuai logika Rust.  
3. Data hasil pengendalian dikirim ke ThingsBoard melalui MQTT untuk pemantauan dan logging.  

---

## 🧠 Teknologi yang Digunakan
| Komponen | Fungsi | Keterangan |
|-----------|---------|------------|
| **ESP32-S3** | Mikrokontroler utama | Mendukung konektivitas Wi-Fi untuk komunikasi IoT |
| **Rust (no_std + embassy)** | Bahasa pemrograman embedded | Untuk kontrol real-time yang aman dan efisien |
| **DWSIM** | Simulator proses industri | Sebagai model sistem fisik untuk uji kontrol |
| **ThingsBoard Cloud** | Platform IoT | Visualisasi data, telemetry, dan remote monitoring |
| **TCP Socket / MQTT Protocol** | Komunikasi antar node | Menghubungkan DWSIM ↔ ESP32 ↔ Cloud |

---

## 🗂️ Struktur Folder
```

SKT12-ALDI-ALAM/
├── backend/              # Kode utama Rust (TCP server/client, kontrol logika)
│   ├── src/
│   │   ├── main.rs
│   │   └── serial.rs
│   ├── Cargo.toml
│   └── Cargo.lock
├── dwsim.py              # Integrasi komunikasi dengan simulator DWSIM
├── diagrams/             # Diagram arsitektur & dokumentasi sistem
└── README.md             # Dokumentasi proyek

````

---

## 🚀 Cara Menjalankan Sistem

### 1️⃣ Jalankan DWSIM
- Buka simulasi proses (misalnya reaktor atau kolom distilasi).  
- Pastikan DWSIM mengirim data melalui TCP port yang sesuai.

### 2️⃣ Jalankan Backend (Rust)
```bash
cd backend
cargo run
````

Kode Rust akan membaca data dari DWSIM melalui TCP dan mengirimkan hasilnya ke ThingsBoard.

### 3️⃣ Visualisasi di ThingsBoard

Masuk ke dashboard ThingsBoard Cloud → buka *Device → Latest Telemetry*.
Data sensor dan status kontrol akan muncul secara real-time.

---
