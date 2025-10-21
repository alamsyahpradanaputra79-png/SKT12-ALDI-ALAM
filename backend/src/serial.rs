use std::io::{BufRead, BufReader};
use std::time::{Duration, SystemTime, UNIX_EPOCH, Instant};
use serialport::SerialPort;
use anyhow::{Result, anyhow};
use log::{info, warn, error, debug};

#[derive(Debug, Clone)]
pub struct SensorData {
    pub timestamp: u64,
    pub temperature: f32,
    pub humidity: f32,
    pub motor_status: Option<bool>,
    pub pump_status: Option<bool>,
}

pub struct SerialMonitor {
    port_name: String,
    baud_rate: u32,
}

#[derive(Debug, Default)]
struct RelayStatus {
    motor: Option<bool>,
    pump: Option<bool>,
}

#[derive(Debug)]
struct PendingSensorData {
    temperature: Option<f32>,
    humidity: Option<f32>,
    last_sent_temp: Option<f32>,
    last_sent_hum: Option<f32>,
    last_send_time: Instant,  // Track when we last sent
}

impl Default for PendingSensorData {
    fn default() -> Self {
        Self {
            temperature: None,
            humidity: None,
            last_sent_temp: None,
            last_sent_hum: None,
            last_send_time: Instant::now(),
        }
    }
}

impl SerialMonitor {
    pub fn new(port_name: String, baud_rate: u32) -> Self {
        Self {
            port_name,
            baud_rate,
        }
    }

    pub async fn start_monitoring<F>(&self, mut on_data: F) -> Result<()>
    where
        F: FnMut(SensorData) -> Result<()> + Send + 'static,
    {
        let port_name = self.port_name.clone();
        let baud_rate = self.baud_rate;

        tokio::task::spawn_blocking(move || {
            info!("Starting serial monitor on {} @ {} baud", port_name, baud_rate);

            loop {
                match serialport::new(&port_name, baud_rate)
                    .timeout(Duration::from_millis(15000))
                    .open()
                {
                    Ok(port) => {
                        info!("Serial port {} opened successfully", port_name);

                        if let Err(e) = Self::read_loop(port, &mut on_data) {
                            error!("Serial read loop error: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to open serial port {}: {}", port_name, e);
                    }
                }

                warn!("Serial connection lost. Retrying in 5 seconds...");
                std::thread::sleep(Duration::from_secs(5));
            }
        }).await?
    }

    fn read_loop<F>(port: Box<dyn SerialPort>, on_data: &mut F) -> Result<()>
    where
        F: FnMut(SensorData) -> Result<()>,
    {
        let mut reader = BufReader::new(port);
        let mut line = String::new();
        let mut relay_status = RelayStatus::default();
        let mut pending_data = PendingSensorData::default();

        // Send interval: kirim data setiap 1 detik meskipun tidak berubah
        const SEND_INTERVAL: Duration = Duration::from_secs(1);

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Ok(_) => {
                    let trimmed = line.trim();

                    if !trimmed.is_empty() {
                        debug!("ESP32: {}", trimmed);
                    }

                    // Parse relay status
                    if let Some((motor, pump)) = Self::parse_relay_status(trimmed) {
                        relay_status.motor = Some(motor);
                        relay_status.pump = Some(pump);
                        info!("Relay status updated: Motor={}, Pump={}",
                              if motor { "ON" } else { "OFF" },
                              if pump { "ON" } else { "OFF" });
                    }

                    // Parse temperature (format: "T  = 29.8 °C")
                    if let Some(temp) = Self::parse_temperature(trimmed) {
                        pending_data.temperature = Some(temp);
                        info!("Temperature parsed: {:.1}°C", temp);
                    }

                    // Parse humidity (format: "RH = 73.3 %")
                    if let Some(hum) = Self::parse_humidity(trimmed) {
                        pending_data.humidity = Some(hum);
                        info!("Humidity parsed: {:.1}%", hum);
                    }

                    // Parse structured format (SENSOR_DATA|temp|hum)
                    if let Some(mut sensor_data) = Self::parse_sensor_data(trimmed) {
                        sensor_data.motor_status = relay_status.motor;
                        sensor_data.pump_status = relay_status.pump;

                        info!("Structured sensor data received: T={:.1}°C, H={:.1}%", 
                              sensor_data.temperature, sensor_data.humidity);

                        if let Err(e) = on_data(sensor_data.clone()) {
                            error!("Failed to process sensor data: {}", e);
                        }
                        
                        // Update last sent values
                        pending_data.last_sent_temp = Some(sensor_data.temperature);
                        pending_data.last_sent_hum = Some(sensor_data.humidity);
                        pending_data.last_send_time = Instant::now();
                        // Reset pending data
                        pending_data.temperature = None;
                        pending_data.humidity = None;
                    }
                    // Send data when we have both temperature and humidity
                    else if pending_data.temperature.is_some() && pending_data.humidity.is_some() {
                        let temp = pending_data.temperature.unwrap();
                        let hum = pending_data.humidity.unwrap();
                        
                        let time_since_last_send = pending_data.last_send_time.elapsed();
                        
                        // Kirim jika:
                        // 1. Belum pernah kirim, ATAU
                        // 2. Data berubah signifikan (> 0.05), ATAU
                        // 3. Sudah lewat 30 detik sejak kirim terakhir
                        let data_changed = pending_data.last_sent_temp
                            .map(|last_t| (temp - last_t).abs() > 0.05)
                            .unwrap_or(true)
                            || pending_data.last_sent_hum
                                .map(|last_h| (hum - last_h).abs() > 0.05)
                                .unwrap_or(true);
                        
                        let should_send = pending_data.last_sent_temp.is_none()
                            || data_changed
                            || time_since_last_send >= SEND_INTERVAL;

                        if should_send {
                            let timestamp = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_nanos() as u64;

                            let sensor_data = SensorData {
                                timestamp,
                                temperature: temp,
                                humidity: hum,
                                motor_status: relay_status.motor,
                                pump_status: relay_status.pump,
                            };

                            let reason = if data_changed {
                                "data changed"
                            } else if time_since_last_send >= SEND_INTERVAL {
                                "periodic update"
                            } else {
                                "first reading"
                            };

                            info!("Sending sensor data ({}): T={:.1}°C, H={:.1}%", 
                                  reason, sensor_data.temperature, sensor_data.humidity);

                            if let Err(e) = on_data(sensor_data) {
                                error!("Failed to process sensor data: {}", e);
                            }

                            // Update last sent values
                            pending_data.last_sent_temp = Some(temp);
                            pending_data.last_sent_hum = Some(hum);
                            pending_data.last_send_time = Instant::now();
                        } else {
                            debug!("Skipping send: data unchanged and interval not reached ({}s elapsed)", 
                                   time_since_last_send.as_secs());
                        }

                        // Reset pending data (but keep last_sent values)
                        pending_data.temperature = None;
                        pending_data.humidity = None;
                    }
                }
                Err(e) => {
                    return Err(anyhow!("Serial read error: {}", e));
                }
            }
        }
    }

    // Parse format: "T  = 29.8 °C" or "T = 29.8 °C"
    fn parse_temperature(line: &str) -> Option<f32> {
        if line.starts_with("T ") || line.starts_with("T=") {
            // Extract number between '=' and '°C'
            if let Some(eq_pos) = line.find('=') {
                let after_eq = &line[eq_pos + 1..];
                if let Some(deg_pos) = after_eq.find('°') {
                    let num_str = after_eq[..deg_pos].trim();
                    if let Ok(temp) = num_str.parse::<f32>() {
                        return Some(temp);
                    }
                }
            }
        }
        None
    }

    // Parse format: "RH = 73.3 %" or "RH= 73.3%"
    fn parse_humidity(line: &str) -> Option<f32> {
        if line.starts_with("RH ") || line.starts_with("RH=") {
            // Extract number between '=' and '%'
            if let Some(eq_pos) = line.find('=') {
                let after_eq = &line[eq_pos + 1..];
                if let Some(pct_pos) = after_eq.find('%') {
                    let num_str = after_eq[..pct_pos].trim();
                    if let Ok(hum) = num_str.parse::<f32>() {
                        return Some(hum);
                    }
                }
            }
        }
        None
    }

    // Parse structured format: "SENSOR_DATA|temperature|humidity"
    fn parse_sensor_data(line: &str) -> Option<SensorData> {
        if let Some(stripped) = line.strip_prefix("SENSOR_DATA|") {
            let parts: Vec<&str> = stripped.split('|').collect();
            if parts.len() == 2 {
                if let (Ok(temperature), Ok(humidity)) = (
                    parts[0].parse::<f32>(),
                    parts[1].parse::<f32>(),
                ) {
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as u64;

                    return Some(SensorData {
                        timestamp,
                        temperature,
                        humidity,
                        motor_status: None,
                        pump_status: None,
                    });
                }
            }
        }
        None
    }

    fn parse_relay_status(line: &str) -> Option<(bool, bool)> {
        if let Some(stripped) = line.strip_prefix("RELAY_STATUS|") {
            let parts: Vec<&str> = stripped.split('|').collect();
            if parts.len() == 2 {
                let motor_part = parts[0].strip_prefix("motor:").unwrap_or("");
                let pump_part = parts[1].strip_prefix("pump:").unwrap_or("");

                let motor_on = motor_part == "ON";
                let pump_on = pump_part == "ON";

                return Some((motor_on, pump_on));
            }
        }
        None
    }
}