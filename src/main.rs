use chrono::Utc;
use rumqttc::{Client, MqttOptions, QoS};
use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Write};
use std::time::Duration;

#[macro_use]
extern crate log;

const FILE_PREFIX: &str = "/data/mqtt/firehose";

fn openfile() -> Result<std::fs::File, std::io::Error> {
    let base_filename = format!("{}.{}", FILE_PREFIX, Utc::now().format("%Y-%m-%d"));

    // Try up to 999 times with a .<number> file name instead of appending to the existing
    // one.
    for i in 0..1000 {
        let filename = if i == 0 {
            format!("{}", &base_filename)
        } else {
            format!("{}.{}", &base_filename, i)
        };
        let f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&filename);
        match f {
            Ok(r) => {
                info!("Writing to {}", &filename);
                return Ok(r);
            }
            Err(e) => match e.kind() {
                ErrorKind::AlreadyExists => {}
                _ => {
                    return Err(e);
                }
            },
        }
    }
    Err(Error::new(
        ErrorKind::AlreadyExists,
        format!("Failed to open file {} for writing", base_filename),
    ))
}

fn savetofile() -> Result<(), std::io::Error> {
    let mut mqttoptions = MqttOptions::new("rust-mqtt-hacking", "67.207.77.99", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(30));

    let (mut client, mut connection) = Client::new(mqttoptions, 200);

    let start = Utc::now();
    let mut outfile = openfile()?;
    let mut day = start.date_naive();

    const INFO_INTERVAL: usize = 10000;
    let mut next_info = INFO_INTERVAL;
    let mut last_info = Utc::now();

    for (i, notification) in connection.iter().enumerate() {
        // Time to log progress report?
        if i == next_info {
            let now = Utc::now();
            let secs = (now - start).num_seconds().abs() as usize;
            let elapsed = (now - last_info).num_seconds().abs() as usize;
            info!(
                "Processed {} events in {}s, {} events/s",
                i,
                secs,
                INFO_INTERVAL / elapsed,
            );
            next_info += INFO_INTERVAL;
            last_info = now;
        }
        let now = Utc::now().date_naive();
        // New day, start a new log file
        if now != day {
            day = now;
            // No way to explicitly close a file, but dropping old reference does it
            outfile = openfile()?;
        }
        match notification {
            // Reconnected. Need to subscribe to the feed again.
            Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(rumqttc::ConnAck {
                session_present: _,
                code: rumqttc::ConnectReturnCode::Success,
            }))) => {
                info!("Connection re-established after {} events", i);
                // pskr/filter/band/mode/txcallsign/rxcallsign/txgrid/rxgrid/txdxcc/rxdxcc
                client
                    .subscribe("pskr/filter/v2/#", QoS::AtMostOnce)
                    .unwrap();
            }

            // Normal payload packet, write out to the file (and add newline)
            Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(rumqttc::Publish {
                topic: _,
                payload,
                ..
            }))) => {
                outfile.write_all(&payload)?;
                outfile.write(b"\n")?;
            }

            // Ping or Ping response, do nothing
            Ok(rumqttc::Event::Incoming(rumqttc::Packet::PingResp))
            | Ok(rumqttc::Event::Outgoing(_)) => {}

            // Catch-all, log and continue
            _ => debug!("Event {}: Notification = {:?}", i, notification),
        }
    }
    Ok(())
}

fn main() {
    env_logger::init();

    savetofile().unwrap();
}
