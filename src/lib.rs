use serde::{Deserialize, Serialize};
use serde_json_wasm as serde_json;
use std::time::{SystemTime, UNIX_EPOCH};
use worker::*;

#[derive(Serialize, Deserialize)]
struct Rules {
    version: i32,
    rules: Vec<Rule>,
}

#[derive(Serialize, Deserialize)]
struct Rule {
    ip_cidr: Vec<String>,
    domain: Vec<String>,
    domain_suffix: Vec<String>,
    domain_keyword: Vec<String>,
}

async fn fetch_config(url: &str) -> String {
    let body = match reqwest::get(url).await {
        Ok(response) => response
            .text()
            .await
            .unwrap_or_else(|_| "error".to_string()),
        Err(_) => "error".to_string(),
    };
    body
}

fn yaml_to_json(yaml_config: &str) -> String {
    let mut ip_cidrs = Vec::new();
    let mut ip6_cidrs = Vec::new();
    let mut domain = Vec::new();
    let mut domain_suffix = Vec::new();
    let mut domain_keyword = Vec::new();

    for line in yaml_config.lines() {
        if line.contains("IP-CIDR") && !line.contains("IP-CIDR6") {
            if let Some(domain) = line.split(',').nth(1) {
                ip_cidrs.push(domain.into());
            }
        } else if line.contains("IP-CIDR6") {
            if let Some(domain) = line.split(',').nth(1) {
                ip6_cidrs.push(domain.into());
            }
        } else if line.contains("DOMAIN-SUFFIX") {
            if let Some(domain) = line.split(',').nth(1) {
                domain_suffix.push(domain.into());
            }
        } else if line.contains("DOMAIN")
            && !line.contains("DOMAIN-SUFFIX")
            && !line.contains("DOMAIN-KEYWORD")
        {
            if let Some(dom) = line.split(',').nth(1) {
                domain.push(dom.into());
            }
        } else if line.contains("DOMAIN-KEYWORD") {
            if let Some(keyword) = line.split(',').nth(1) {
                domain_keyword.push(keyword.into());
            }
        }
    }

    let ipcidr_final = ip_cidrs.into_iter().chain(ip6_cidrs).collect();

    let rules = Rules {
        version: 1,
        rules: vec![Rule {
            ip_cidr: ipcidr_final,
            domain,
            domain_suffix,
            domain_keyword,
        }],
    };

    let json_string = serde_json::to_string(&rules).unwrap();
    json_string
}

#[event(fetch)]
async fn main(req: Request, env: Env, _: Context) -> Result<Response> {
    let url = req.path().trim_start_matches('/').to_string();
    let kv = env.kv("SING_BOX_RULE").unwrap();
    let cached_json_time = kv
        .get(req.path().as_str())
        .text()
        .await
        .unwrap()
        .unwrap_or_default();
    let cached_json = kv.get(&url).text().await.unwrap().unwrap_or_default();
    // check if cached_json_time exists
    if cached_json_time.is_empty() || cached_json.is_empty() {
        let yaml_config = fetch_config(&url).await;
        let json_string = yaml_to_json(&yaml_config);
        if let Err(e) = kv.put(&url, &json_string) {
            eprintln!("Failed to put data in KV store: {:?}", e);
        }
        // store current unix timestamp
        if let Err(e) = kv.put(
            &req.path().as_str(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string()
                .as_str(),
        ) {
            eprintln!("Failed to store timestamp in KV store: {:?}", e);
        }
        return Response::ok(json_string);
    }
    // check if json_time vs current time is less than 24 hours
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if let Ok(cached_time) = cached_json_time.parse::<u64>() {
        if current_time - cached_time < 86400 {
            return Response::ok(cached_json);
        }
    }

    let yaml_config = fetch_config(&url).await;
    let json_string = yaml_to_json(&yaml_config);
    if let Err(e) = kv.put(&url, &json_string) {
        eprintln!("Failed to put data in KV store: {:?}", e);
    }
    // store current unix timestamp
    if let Err(e) = kv.put(
        &req.path().as_str(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string()
            .as_str(),
    ) {
        eprintln!("Failed to store timestamp in KV store: {:?}", e);
    }
    Response::ok(json_string)
}
