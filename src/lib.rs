use worker::*;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Serialize, Deserialize)]
struct Rules {
    version: i32,
    rules: Vec<Rule>
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
    
}

#[event(fetch)]
async fn main(req: Request, _: Env, _: Context) -> Result<Response> {
    //use reqwest to get the content of the url 
    let url = req.path().trim_start_matches("/").to_string();
    let res = reqwest::get(url).await;
    let body = match res {
        Ok(res) => res.text().await.unwrap_or_else(|_| "error".to_string()),
        Err(_) => "error".to_string()
    };
    //read every line of body and convert yaml to json 
    let ip_cidrs: Vec<String> = body.lines()
        .filter(|line| line.contains("IP-CIDR") && !line.contains("IP-CIDR6"))
        .map(|line| line
            .trim_start_matches("  - IP-CIDR,")
            .trim_end_matches(",no-resolve")
            .to_string())
        .collect();
    let ip6_cidrs: Vec<String> = body.lines()
        .filter(|line| line.contains("IP-CIDR6"))
        .map(|line| line
            .trim_start_matches("  - IP-CIDR6,")
            .trim_end_matches(",no-resolve")
            .to_string())
        .collect();

    let domain_suffix: Vec<String> = body.lines()
        .filter(|line| line.contains("DOMAIN-SUFFIX"))
        .map(|line| line
            .split(',')           // Split at comma
            .nth(1)              // Take first part
            .unwrap_or("")       // Handle potential None
            .to_string())
        .collect();

    let domain: Vec<String> = body.lines()
        .filter(|line| line.contains("DOMAIN") && !line.contains("DOMAIN-SUFFIX") && !line.contains("DOMAIN-KEYWORD"))
        .map(|line| line
            .split(',')           // Split at comma
            .nth(1)               // Take first part
            .unwrap_or("")       // Handle potential None
            .to_string())
        .collect();

    let domain_keyword: Vec<String> = body.lines()
        .filter(|line| line.contains("DOMAIN-KEYWORD"))
        .map(|line| line
            .split(',')           // Split at comma
            // Take second part 
            .nth(1)
            .unwrap_or("")       // Handle potential None
            .to_string())
        .collect();
    
    let ipcidr_final: Vec<String> = ip_cidrs.iter().chain(ip6_cidrs.iter()).cloned().collect(); 

    let rules = Rules {
        version: 1,
        rules: vec![Rule { ip_cidr: ipcidr_final, domain: domain, domain_suffix: domain_suffix, domain_keyword: domain_keyword}]
    };
    let json_string = serde_json::to_string(&rules)?;

    Response::ok(format!("{}", json_string))
}