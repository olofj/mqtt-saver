use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Write};
use std::time::Duration;

#[macro_use]
extern crate log;

const FILE_PREFIX: &str = "/data/mqtt/firehose";

// Interval at which we log progress / rate
const INFO_INTERVAL: usize = 10000;

// Open a file for writting the output to. Attempt to create .<index> files
// instead of continuing on the same file, to make re-opens/drops more obvious
// when looking at the file contents.

fn openfile() -> Result<std::fs::File, std::io::Error> {
    let base_filename = format!("{}.{}", FILE_PREFIX, Utc::now().format("%Y-%m-%d"));

    for i in 0..1000 {
        let filename = if i == 0 {
            format!("{}", &base_filename)
        } else {
            format!("{}.{}", &base_filename, i)
        };

        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&filename)
        {
            Ok(f) => {
                info!("Writing to {}", &filename);
                return Ok(f);
            }
            Err(e) => {
                if e.kind() != ErrorKind::AlreadyExists {
                    // Actual error, not just "file exists". Bubble it up.
                    return Err(e);
                }
            }
        }
    }

    // If we can't open by now, give up and return error
    Err(Error::new(
        ErrorKind::AlreadyExists,
        format!("Failed to open file {} for writing", base_filename),
    ))
}

fn savetofile() -> Result<(), std::io::Error> {
    let client_id: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .map(char::from)
        .collect();

    let mut mqttoptions = MqttOptions::new(client_id, "mqtt.pskreporter.info", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(30));

    let (mut client, mut connection) = Client::new(mqttoptions, 200000);

    let start = Utc::now();
    let mut outfile = openfile()?;
    let mut day = start.date_naive();

    let mut next_info = INFO_INTERVAL;
    let mut last_info = Utc::now();
    let mut written = 0;

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

        // New day, start a new log file
        if day != Utc::now().date_naive() {
            day = Utc::now().date_naive();
            // No way to explicitly close a file, but dropping old reference does it
            outfile = openfile()?;
            written = 0;
        }

        match notification {
            // (Re)connected. Need to subscribe to the feed.
            Ok(Event::Incoming(Packet::ConnAck(rumqttc::ConnAck {
                session_present: _,
                code: rumqttc::ConnectReturnCode::Success,
            }))) => {
                info!("Connection re-established after {} events", i);
                // pskr/filter/band/mode/txcallsign/rxcallsign/txgrid/rxgrid/txdxcc/rxdxcc
                client
                    .subscribe("pskr/filter/v2/#", QoS::AtMostOnce)
                    .unwrap();
            }

            // Subscription ack, start a new output file if the previous
            // one was written to (in case of server format changes, etc)
            Ok(Event::Incoming(Packet::SubAck(rumqttc::SubAck { .. }))) => {
                if written > 0 {
                    outfile = openfile()?;
                    written = 0;
                }
            }

            // Normal payload packet, write out to the file (and add newline)
            Ok(Event::Incoming(Packet::Publish(rumqttc::Publish {
                topic: _, payload, ..
            }))) => {
                outfile.write_all(&payload)?;
                outfile.write(b"\n")?;
                written += 1;
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
