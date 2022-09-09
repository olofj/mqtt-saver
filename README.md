# mqtt-saver
Rust hack to save mqtt pskreporter feed to disk

To build container
```
docker build -t mqtt_saver .
```

To run:
```
docker run -b -v /data:/data mqtt_saver
docker logs -f <containerid>
```

# PSKreporter MQTT feed
For more information, see http://mqtt.pskreporter.info/

