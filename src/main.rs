use chrono::Utc;
use rumqttc::{Client, MqttOptions, QoS};
use std::fs::OpenOptions;
use std::io::Write;
use std::time::Duration;

fn savetofile() -> Result<(), std::io::Error> {
    let mut mqttoptions = MqttOptions::new("rust-mqtt-hacking", "67.207.77.99", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (mut client, mut connection) = Client::new(mqttoptions, 10);
    // pskr/filter/band/mode/txcallsign/rxcallsign/txgrid/rxgrid/txdxcc/rxdxcc
    client.subscribe("pskr/firehose", QoS::AtMostOnce).unwrap();

    let now = Utc::now();
    let filename = format!("/home/olof/mqtt/firehose.{}", now.format("%Y-%m-%d"));
    println!("Writing to {}", filename);
    let mut outfile = OpenOptions::new()
        .append(true)
        .create(true)
        .open(filename)?;
    let mut day = now.date_naive();

    for (_i, notification) in connection.iter().enumerate() {
        let now = Utc::now().date_naive();
        if now != day {
            // New day, new file
            day = now;
            // No way to explicitly close a file, but dropping old reference does it
            let filename = format!("/home/olof/mqtt/firehose.{}", now.format("%Y-%m-%d"));
            println!("Writing to {} now", filename);
            outfile = OpenOptions::new()
                .append(true)
                .create(true)
                .open(filename)?;
        }
        match notification {
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
            _ => println!("Notification = {:?}", notification),
        }
    }
    Ok(())
}

fn main() {
    savetofile().unwrap();
}
