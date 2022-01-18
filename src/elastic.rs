use reqwest::{Client};
use crate::configure::ElkConf;
#[derive(Debug,Deserialize)]
pub struct Counter {
    pub count: u32,
}
#[derive(Debug)]
pub enum ElkError {
    Request,
    UnexpectedResponse,
}


pub async fn elk_build(bodyq: &serde_json::Value, elk: &ElkConf) -> Result<Counter, ElkError> {
    log::debug!("Request for: {}",bodyq);
    let resp = Client::new()
    .post(&elk.url).header("Authorization", &elk.authorization)
    .json(&bodyq).send().await;
    match resp {
        Ok(e) => {
            if e.status().is_success() {
                let s: Counter = e.json().await.unwrap();
                log::debug!("Status: {:?}", s);
                Ok(s)
            } else {
                let s  = &e.text().await.unwrap();
                log::error!("Error resp: {:?}", s);
                Err(ElkError::UnexpectedResponse)
            }
        },
        Err(e) => {
            log::error!("Error request: {:?}", e);
            Err(ElkError::Request)
        },
        
    }
    
}
