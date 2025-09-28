use worker::*;
use serde::{Deserialize, Serialize};
use serde_json_wasm as serde_json;

#[derive(Serialize, Deserialize)]
struct Rules {
    version: i32,
    rules: Vec<Rule>,
}

#[derive(Serialize, Deserialize)]
struct Rule {
    #[serde(default)]
    ip_cidr: Vec<String>,
    #[serde(default)]
    domain: Vec<String>,
    #[serde(default)]
    domain_suffix: Vec<String>,
    #[serde(default)]
    domain_keyword: Vec<String>,
}

#[event(fetch)]
async fn main(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    // 1. Get URL
    let path = req.path();
    let url_str = path.trim_start_matches('/');
    if url_str.is_empty() {
        console_error!("Missing URL in path"); // Log error
        return Response::error("Missing URL in path", 400);
    }

    // 2. Fetch
    let url = match worker::Url::parse(url_str) {
        Ok(parsed_url) => parsed_url,
        Err(e) => {
            console_error!("Invalid URL '{}': {}", url_str, e); // Log parse error
            return Response::error(format!("Invalid URL: {}", e), 400);
        }
    };
    let mut response = match Fetch::Url(url).send().await {
        Ok(resp) => resp,
        Err(e) => {
            console_error!("Fetch failed for URL '{}': {}", url_str, e); // Log fetch error
            return Response::error(format!("Failed to fetch URL: {}", e), 502);
        }
    };

    let status = response.status_code();

    // 3. Read Body (read even if status is not success for debugging)
    let body = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            console_error!("Failed to read response body for URL '{}': {}", url_str, e); // Log read error
            return Response::error(format!("Failed to read response body: {}", e), 500);
        }
    };

    // Check status *after* attempting to read body
     if !(200..=299).contains(&status) {
         console_error!(
             "Remote server error for '{}': Status {}",
             url_str,
             status
         );
         return Response::error(format!("Remote server returned status: {}", status), 502);
     }


    // 4. Parse - Pre-allocate vectors based on estimated content
    let estimated_rules = body.lines().count() / 10; // Rough estimate
    let mut ipcidr_final = Vec::with_capacity(estimated_rules);
    let mut domain = Vec::with_capacity(estimated_rules / 4);
    let mut domain_suffix = Vec::with_capacity(estimated_rules / 4);
    let mut domain_keyword = Vec::with_capacity(estimated_rules / 4);

    for line in body.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
            continue;
        }


        let parts: Vec<&str> = trimmed_line.split(',').map(str::trim).collect();

        if parts.len() >= 2 {
            let rule_type = parts[0];
            let value = parts[1];

            match rule_type {
                "- IP-CIDR" | "- IP-CIDR6" | "IP-CIDR" | "IP-CIDR6" => {
                    ipcidr_final.push(value.to_string());
                }
                "- DOMAIN-SUFFIX" | "DOMAIN-SUFFIX" => {
                    domain_suffix.push(value.to_string());
                }
                "- DOMAIN-KEYWORD" | "DOMAIN-KEYWORD" => {
                    domain_keyword.push(value.to_string());
                }
                "- DOMAIN" | "DOMAIN" => {
                    domain.push(value.to_string());
                }
                _ => {}
            }
        }
    }


    // 5. Construct & Serialize
    let rules = Rules {
        version: 1,
        rules: vec![Rule {
            ip_cidr: ipcidr_final,
            domain,
            domain_suffix,
            domain_keyword,
        }],
    };

    let json_string = match serde_json::to_string(&rules) {
         Ok(json) => json,
         Err(e) => {
            console_error!("Failed to serialize JSON: {}", e); // Log serialize error
            return Response::error(format!("Failed to serialize JSON: {}", e), 500);
         }
    };

    // 6. Respond
    let mut headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    let mut resp = Response::ok(json_string)?;
    resp.headers_mut().set("Content-Type", "application/json")?;
    Ok(resp)
}
