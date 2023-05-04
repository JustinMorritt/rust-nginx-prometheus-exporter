# NGINX PROMETHEUS EXPORTER


## Pre-Requisites
sudo vim /etc/nginx/nginx.conf

log_format logger-json escape=json '{"source": "nginx", "time": $msec, "resp_body_size": $body_bytes_sent, "host": "$http_host", "address": "$remote_addr", "request_length": $request_length, "method": "$request_method", "uri": "$request_uri", "status": $status,  "user_agent": "$http_user_agent", "resp_time": $request_time, "upstream_addr": "$upstream_addr"}';

server {
    listen 127.0.0.1:80;
    server_name 127.0.0.1;

    location /nginx_status {
        stub_status;
    }
}

# Node Exporter [Guide](https://prometheus.io/docs/guides/node-exporter/)
[Better guide here](https://ourcodeworld.com/articles/read/1686/how-to-install-prometheus-node-exporter-on-ubuntu-2004)
```terminal
   sudo cp rust-nginx-exporter /usr/local/bin/
  
   sudo useradd --no-create-home --shell /bin/false node_exporter
   sudo chown node_exporter:node_exporter /usr/local/bin/node_exporter

   sudo vim /etc/systemd/system/rust-nginx-exporter.service
   sudo systemctl daemon-reload
   sudo systemctl enable rust-nginx-exporter
   sudo systemctl start rust-nginx-exporter
   sudo systemctl status rust-nginx-exporter
```

## Create /etc/systemd/system/rust-nginx-exporter.service
[Unit]
Description=Nginx Exporter Service
After=network.target
[Service]
User=root
Group=root
Type=simple
ExecStart=/usr/local/bin/rust-nginx-exporter "/var/log/nginx/access.log"
[Install]
WantedBy=multi-user.target


# HELP Nginx Log_http_requests Number of HTTP requests received.
# TYPE Nginx Log_http_requests counter
Nginx Log_http_requests_total{method=\"GET\",path=\"/\"} 5
Nginx Log_http_requests_total{method=\"GET\",path=\"/sitesettings/readCurrent\"} 2
Nginx Log_http_requests_total{method=\"POST\",path=\"/user/readAll\"} 1
Nginx Log_http_requests_total{method=\"POST\",path=\"/user/login\"} 1
Nginx Log_http_requests_total{method=\"POST\",path=\"/sitesettings/readAll\"} 1
# EOF

rustup target add x86_64-unknown-linux-musl
cargo build --target=x86_64-unknown-linux-musl

rustup target add x86_64-unknown-linux-gnu
cargo build --target=x86_64-unknown-linux-gnu

[target.x86_64-unknown-linux-musl]
linker = "rust-lld"


localhost:9200/metrics