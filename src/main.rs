use chrono::Utc;
use rumqttc::{Client, MqttOptions, MqttState, QoS, StateError};
use std::fs::OpenOptions;
use std::io::Write;
use std::time::Duration;

#[macro_use]
extern crate log;

fn savetofile() -> Result<(), std::io::Error> {
    let mut mqttoptions = MqttOptions::new("rust-mqtt-hacking", "67.207.77.99", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(30));

    let (mut client, mut connection) = Client::new(mqttoptions, 200);

    let start = Utc::now();
    let filename = format!("/home/olof/mqtt/firehose.{}", start.format("%Y-%m-%d"));
    info!("Writing to {}", filename);
    let mut outfile = OpenOptions::new()
        .append(true)
        .create(true)
        .open(filename)?;
    let mut day = start.date_naive();

    const INFO_INTERVAL: usize = 10000;
    let mut next_info = INFO_INTERVAL;
    let mut last_info = Utc::now();

    for (i, notification) in connection.iter().enumerate() {
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
        if now != day {
            // New day, new file
            day = now;
            // No way to explicitly close a file, but dropping old reference does it
            let filename = format!("/home/olof/mqtt/firehose.{}", now.format("%Y-%m-%d"));
            info!("Writing to {} now", filename);
            outfile = OpenOptions::new()
                .append(true)
                .create(true)
                .open(filename)?;
        }
        match notification {
            Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(rumqttc::ConnAck {
                session_present: _,
                code: Success,
            }))) => {
                // pskr/filter/band/mode/txcallsign/rxcallsign/txgrid/rxgrid/txdxcc/rxdxcc
                client
                    .subscribe("pskr/filter/v2/#", QoS::AtMostOnce)
                    .unwrap();
            }
            Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(rumqttc::Publish {
                topic: _,
                payload,
                ..
            }))) => {
                outfile.write_all(&payload)?;
                outfile.write(b"\n")?;
            }
            Ok(rumqttc::Event::Incoming(rumqttc::Packet::PingResp))
            | Ok(rumqttc::Event::Outgoing(_)) => {}
            _ => debug!("Notification = {:?}", notification),
        }
    }
    Ok(())
}

fn main() {
    env_logger::init();

    savetofile().unwrap();
}
