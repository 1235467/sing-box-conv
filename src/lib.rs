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
    console_log!("Attempting to fetch URL: {}", url_str); // <-- LOG URL

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
    console_log!("Received status {} for URL: {}", status, url_str); // <-- LOG STATUS

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
             "Remote server error for '{}': Status {}, Body sample: '{}'",
             url_str,
             status,
             body.chars().take(200).collect::<String>() // Log first 200 chars of bad response
         );
         return Response::error(format!("Remote server returned status: {}", status), 502);
     }

    // <-- LOG THE RAW BODY
    console_log!(
        "Raw body received (first 500 chars) for '{}':\n{}",
        url_str,
        body.chars().take(500).collect::<String>()
    );

    // 4. Parse
    let mut ipcidr_final = Vec::new();
    let mut domain = Vec::new();
    let mut domain_suffix = Vec::new();
    let mut domain_keyword = Vec::new();
    let mut processed_lines = 0; // Counter for debugging

    for line in body.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
            continue;
        }

        // <-- LOGGING INSIDE LOOP
        // console_log!("Processing line: '{}'", trimmed_line); // Can be noisy, enable if needed

        if let Some((rule_type_raw, value_raw)) = trimmed_line.split_once(',') {
            let rule_type = rule_type_raw.trim();
            let value = value_raw.trim().to_string(); // Trim and own

             // <-- LOGGING SPLIT
             // console_log!("Split OK: type='{}', value='{}'", rule_type, value); // Enable if needed

            match rule_type {
                "- IP-CIDR" | "- IP-CIDR6" => { // Combine match arms
                    // console_log!("Matched IP-CIDR/6: {}", value); // Enable if needed
                    ipcidr_final.push(value);
                    processed_lines += 1;
                }
                "- DOMAIN-SUFFIX" => {
                    // console_log!("Matched DOMAIN-SUFFIX: {}", value); // Enable if needed
                    domain_suffix.push(value);
                    processed_lines += 1;
                }
                "- DOMAIN-KEYWORD" => {
                     // console_log!("Matched DOMAIN-KEYWORD: {}", value); // Enable if needed
                    domain_keyword.push(value);
                    processed_lines += 1;
                }
                "- DOMAIN" => {
                    // console_log!("Matched DOMAIN: {}", value); // Enable if needed
                    domain.push(value);
                    processed_lines += 1;
                }
                _ => {
                    // <-- LOG UNMATCHED TYPES
                    console_log!("Unknown rule type found: '{}' on line: '{}'", rule_type, trimmed_line);
                 }
            }
        } else {
            // <-- LOG LINES THAT DIDN'T SPLIT
            console_log!("Line did not contain ',': '{}'", trimmed_line);
        }
    }

    console_log!("Finished processing. Found {} matching rules.", processed_lines); // <-- LOG COUNT

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