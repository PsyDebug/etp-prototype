use serde_json::json;
use serde_yaml;
use serde_yaml::Mapping;
use serde_yaml::Value;

#[derive(Debug, Serialize, Deserialize)]
struct Configure {
    tasks: Vec<Task>,
    elk: ElkConf,
    server: ServerConf,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ElkConf {
    pub url: String,
    pub authorization: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConf {
    pub bind: String,
    pub metric_path: String,
}
#[derive(Debug)]
pub struct WorkerConf {
    pub period: u32,
    pub metric_name: String,
    pub description: String,
    pub environment: String,
    pub request: serde_json::Value,
}
#[derive(Debug, Serialize, Deserialize)]
struct Task {
    metric_name: String,
    period: u32,
    description: String,
    environment: String,
    filter: Vec<Mapping>,
    must_not: Option<Vec<Mapping>>,
}
#[derive(Debug, Serialize, Deserialize)]
struct Request {
    filter: Vec<Mapping>,
    must_not: Option<Vec<Mapping>>,
}
#[derive(Debug, Serialize, Deserialize)]
struct Query {
    bool: Request,
}
#[derive(Debug, Serialize, Deserialize)]
struct BuildRequest {
    query: Query,
}
#[derive(Debug)]
pub struct Settings {
    pub worker: Vec<WorkerConf>,
    pub elk: ElkConf,
    pub server: ServerConf,
}

impl Settings {
    pub fn new(filepath: String) -> Result<Settings, Box<dyn std::error::Error>> {
        println!("Load config: {}", &filepath);
        let f = std::fs::File::open(filepath)?;
        let d: Configure = serde_yaml::from_reader(f)?;
        let worker: Vec<WorkerConf> = restructurizer(&d.tasks);
        let settings = Settings {
            worker: worker,
            elk: d.elk,
            server: d.server,
        };
        Ok(settings)
    }
}

fn restructurizer(tasks: &Vec<Task>) -> Vec<WorkerConf> {
    let mut worker = Vec::new();
    for task in tasks {
        let mut newfilter = task.filter.clone();
        newfilter.push(tofilter(&task.period));
// TODO
        let request = BuildRequest {
            query: Query {
                bool: Request {
                    filter: newfilter,
                    must_not: task.must_not.clone(),
                },
            },
        };
        worker.push(WorkerConf {
            period: task.period,
            metric_name: task.metric_name.clone(),
            description: task.description.clone(),
            environment: task.environment.clone(),
            request: json!(&request),
        });
    }
    worker
}

// TODO
fn tofilter(period: &u32) -> Mapping {
    let mut timestamp: Mapping = Mapping::new();
    timestamp.insert(
        Value::String("gte".to_string()),
        Value::String("now-".to_string() + &period.to_string() + "m"),
    );
    timestamp.insert(
        Value::String("lte".to_string()),
        Value::String("now".to_string()),
    );
    let mut range = Mapping::new();
    range.insert(
        Value::String("@timestamp".to_string()),
        Value::Mapping(timestamp),
    );
    let mut tofilter = Mapping::new();
    tofilter.insert(Value::String("range".to_string()), Value::Mapping(range));
    tofilter
}
