use worker::*;
use serde::{Deserialize, Serialize};
use serde_json;

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

#[derive(Serialize, Deserialize)]
struct Domain {
    // Assuming you need to fill this in later
}

#[event(fetch)]
async fn main(req: Request, _: Env, _: Context) -> Result<Response>   {
    let url = req.path().trim_start_matches('/').to_string();
    let body = match reqwest::get(&url).await {
        Ok(response) => response.text().await.unwrap_or_else(|_| "error".to_string()),
        Err(_) => "error".to_string(),
    };

    let mut ip_cidrs = Vec::new();
    let mut ip6_cidrs = Vec::new();
    let mut domain = Vec::new();
    let mut domain_suffix = Vec::new();
    let mut domain_keyword = Vec::new();

    for line in body.lines() {
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
        } else if line.contains("DOMAIN") && !line.contains("DOMAIN-SUFFIX") && !line.contains("DOMAIN-KEYWORD") {
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

    let json_string = serde_json::to_string(&rules)?;
    Response::ok(json_string)
}