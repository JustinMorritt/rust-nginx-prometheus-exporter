use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    sync::{atomic::AtomicU64, RwLock},
};

// Alternative metrics library https://docs.rs/metrics-exporter-prometheus/0.12.0/metrics_exporter_prometheus/index.html#
use prometheus_client::{
    encoding::{text::encode, EncodeLabelSet, EncodeLabelValue},
    metrics::{counter::Counter, family::Family, histogram::Histogram},
    registry::Registry,
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Metrics {
    access_log_path: String,
    registry: Registry,
    http_requests: Family<Labels, Counter>,
    http_bytes_sent: Family<Labels, Counter>,
    http_request_bytes: Family<Labels, Counter>,
    http_request_time: Family<Labels, Histogram>,
    http_status: Histogram,
    parse_errors: Counter<u64, AtomicU64>,

    // FILE OPERATIONS
    last_log_file_size: RwLock<u64>,
    last_log_file_line_count: RwLock<usize>,
    last_log_file_line_index: RwLock<usize>,
}

impl Metrics {
    pub fn new(access_log_path: String, prefix: String) -> Metrics {
        let mut registry = if String::is_empty(&prefix) {
            Registry::default()
        } else {
            Registry::with_prefix(prefix)
        };

        let http_requests = Family::<Labels, Counter>::default();
        let http_bytes_sent = Family::<Labels, Counter>::default();
        let http_request_bytes = Family::<Labels, Counter>::default();
        let http_request_time = Family::<Labels, Histogram>::new_with_constructor(|| {
            let custom_buckets = [
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ];
            Histogram::new(custom_buckets.into_iter())
        });
        let custom_buckets = [200.0, 300.0, 400.0, 500.0];
        let http_status = Histogram::new(custom_buckets.into_iter());
        let parse_errors = Counter::<u64, AtomicU64>::default();

        registry.register(
            "http_requests",
            "Number of HTTP requests received",
            http_requests.clone(),
        );
        registry.register(
            "body_bytes_sent",
            "Number of bytes sent",
            http_bytes_sent.clone(),
        );
        registry.register(
            "request_length",
            "Number of bytes in request",
            http_request_bytes.clone(),
        );
        registry.register(
            "parse_errors",
            "Number of parsing errors",
            parse_errors.clone(),
        );
        registry.register("request_time", "request_time", http_request_time.clone());
        registry.register("request_status", "request_status", http_status.clone());

        Metrics {
            access_log_path,
            registry,
            http_requests,
            http_bytes_sent,
            http_request_bytes,
            http_request_time,
            http_status,
            parse_errors,
            last_log_file_size: RwLock::new(0),
            last_log_file_line_count: RwLock::new(0),
            last_log_file_line_index: RwLock::new(0),
        }
    }

    pub fn record_metrics(&self) {
        if let Ok(Some(lines)) = self.read_lines() {
            let mut skip_index = 0;
            if let Ok(val) = self.last_log_file_line_index.read() {
                skip_index = *val
            }

            for (index, line) in lines.enumerate().skip(skip_index) {
                if let Ok(mut val) = self.last_log_file_line_index.write() {
                    *val = index + 1;
                }
                match line {
                    Ok(text) => match serde_json::from_str::<NginxLogLine>(&text) {
                        Ok(log_line) => {
                            let method = match log_line.method.as_str() {
                                "GET" => Method::GET,
                                "POST" => Method::POST,
                                "PUT" => Method::PUT,
                                "PATCH" => Method::PATCH,
                                "OPTIONS" => Method::OPTIONS,
                                "DELETE" => Method::DELETE,
                                _ => Method::OTHER,
                            };

                            let labels = Labels {
                                method: method,
                                status: log_line.status.to_string(),
                            };

                            self.http_requests.get_or_create(&labels).inc();
                            self.http_bytes_sent
                                .get_or_create(&labels)
                                .inc_by(log_line.resp_body_size);
                            self.http_request_bytes
                                .get_or_create(&labels)
                                .inc_by(log_line.request_length);
                            self.http_request_time
                                .get_or_create(&labels)
                                .observe(log_line.resp_time);
                            self.http_status.observe(log_line.status as f64);

                            //println!("LogLine:\n{:?}", log_line);
                        }
                        Err(err) => println!("LogLine Error:\n{:?}", err),
                    },
                    Err(_) => {
                        self.parse_errors.inc();
                    }
                }
            }
        }
    }

    pub fn _print_metrics(&self) {
        let mut encoded = String::new();
        encode(&mut encoded, &self.registry).unwrap();

        println!("Scrape output:\n");
        let split: Vec<&str> = encoded.split('\n').collect();
        for line in split.iter() {
            println!("{:?}", line);
        }
    }

    pub async fn render(&self) -> String {
        let mut encoded = String::new();
        if let Ok(_) = encode(&mut encoded, &self.registry) {
            return encoded;
        }
        return String::from("Error Encoding");
    }

    fn read_lines(&self) -> Result<Option<io::Lines<BufReader<File>>>, std::io::Error> {
        let access_log_file = File::open(&self.access_log_path)?;

        let mut should_read_more_lines = false;
        if let Ok(data) = access_log_file.metadata() {
            let len = data.len();
            if let Ok(mut val) = self.last_log_file_size.write() {
                if *val != len {
                    should_read_more_lines = true;
                    if *val > len {
                        // When the file shrinks / gets log rotated we want to reset our index to 0
                        if let Ok(mut val) = self.last_log_file_line_index.write() {
                            *val = 0
                        }
                    }
                    *val = len;
                    println!("Recorded File Length: {:?}", len);
                }
            }
        }

        if should_read_more_lines {
            let access_log_file2 = File::open(&self.access_log_path)?;
            let access_log_reader = io::BufReader::new(access_log_file);
            let access_log_reader2 = io::BufReader::new(access_log_file2);

            let lines = access_log_reader.lines();
            let count = access_log_reader2.lines().count();
            if let Ok(mut val) = self.last_log_file_line_count.write() {
                *val = count
            }
            return Ok(Some(lines));
        }

        return Ok(None);
    }
}

/*
{
    "source": "nginx",
    "time": 1658188800011,
    "resp_body_size": 23615,
    "host": "api.company.com",
    "address": "192.20.0.1",
    "request_length": 482,
    "method": "POST",
    "uri": "/service/route?variant=default",
    "status": 200,
    "user_agent": "Apache-HttpClient/4.5.1 (Java/11.0.15)",
    "resp_time": 0.042,
    "upstream_addr": "10.0.0.20:80"
}
 */
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct NginxLogLine {
    source: String,
    time: f64,
    resp_body_size: u64,
    host: String,
    address: String,
    request_length: u64,
    method: String,
    uri: String,
    status: u64,
    user_agent: String,
    resp_time: f64,
    upstream_addr: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
enum Method {
    GET,
    PUT,
    POST,
    PATCH,
    OPTIONS,
    DELETE,
    OTHER,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct Labels {
    method: Method,
    status: String,
}
