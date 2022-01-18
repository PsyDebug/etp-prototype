use std::{time, env};
use std::io::Write;
use std::sync::Arc;
use std::net::ToSocketAddrs;
use chrono::Local;
use env_logger::Builder;
use log::LevelFilter;
use warp::Filter;
use prometheus::{Opts, Registry, Counter, TextEncoder, Encoder};
use tokio::task;
use etp::configure::Settings;



fn main() -> () {
  Builder::new().filter_level(LevelFilter::Info).parse_env("LOGLEVEL")
  .format(|buf, record| {
    writeln!(buf,
      "{} [{}] - {}",
      Local::now().format("%Y-%m-%dT%H:%M:%S"),
      record.level(),
      record.args()
    )
  })
  .init();
  let args: Vec<String> = env::args().collect();
  let path = match args.get(1) {
    Some(p) => p.to_string(),
    None => "config.yml".to_string(),
  };
  let d = Settings::new(path);
  let config = match d {
    Ok(e) => e,
    Err(e) => panic!("Configure error {:#?}", e),
  };
  start_app(config);
}


#[tokio::main(flavor = "multi_thread")]
async fn start_app(config: Settings) {
  let mut children = Vec::new();
  
  let tasks=config.worker;
  let elk = Arc::new(config.elk);
  let bind_addr = match config.server.bind.to_socket_addrs() {
    Ok(mut e) => e.next().unwrap(),
    Err(e) => {
      log::error!("Server bind (server.bind) Will be set default 127.0.0.1:8080 {}", e);
      "127.0.0.1:8080".to_socket_addrs().unwrap().next().unwrap()
    },
  };
  log::info!("Start {} tasks", &tasks.len());
  log::debug!("{:#?}", &tasks);
  let r = Registry::new();
  let counter_bad_opts = Opts::new("etp_bad_request","bad elk request").clone();
  let counter_bad = Counter::with_opts(counter_bad_opts).unwrap();
  r.register(Box::new(counter_bad.clone())).unwrap();

  for task in tasks {
    let request = Arc::new(task.request);
    let metric_name = task.metric_name;
    let period = task.period;
    let description = task.description;
    let counter_opts = Opts::new(&metric_name, &description)
    .const_label("period", &period.to_string())
    .const_label("environment", &task.environment.to_string());
    let counter = Counter::with_opts(counter_opts).unwrap();
    r.register(Box::new(counter.clone())).unwrap();

    let c2 = counter.clone();
    let c2_bad = counter_bad.clone();

    let request = request.clone();
    let elk = elk.clone();
    let child = task::spawn(async move {
      loop {
        // request to elk
        // THIS
        match etp::elastic::elk_build(&request, &elk).await {
          Ok(e) => {
            log::info!("For metric {} count: {}",&metric_name,&e.count);
            c2.inc_by(e.count as f64);
          },     // add increment to metric
          Err(e) => {
            log::warn!("Elastic error: {:#?}", e);
            c2_bad.inc_by(1.0)}, // add increment for bad req
        };
        log::debug!("Sleep {}", metric_name);
        tokio::time::sleep(time::Duration::from_secs((period*60).into())).await;
        log::debug!("Thread {} finish", metric_name);
      }
    });
    children.push(child);
  }
  
  
  let routes = warp::path(config.server.metric_path).map(move ||{
    let mut buffer = Vec::<u8>::new();
    let encoder = TextEncoder::new();
    let metric_families = r.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap(); 
    String::from_utf8(buffer.clone()).unwrap()
  } );
  
  // TODO
  
  warp::serve(routes).run(bind_addr).await;
  
}
