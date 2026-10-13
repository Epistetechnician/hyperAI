//! HTTP compatibility, authentication, origin, request, and response checks.

use std::{error::Error, path::PathBuf, time::Duration};

use hyperai_local_service::{
    AuthState,
    lifecycle::{self, ServiceRecordV1},
    server,
    storage::{self, HomePaths},
};
use hyperai_market::{LocalMarketHistoryPageV1, verify_local_market_history_page_v1};
use reqwest::{Client, header};
use serde_json::Value;
use tokio::task::JoinHandle;

async fn launch(
    home: PathBuf,
) -> Result<
    (
        JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>,
        HomePaths,
        ServiceRecordV1,
        String,
    ),
    Box<dyn Error + Send + Sync>,
> {
    let server_home = home.clone();
    let task = tokio::spawn(async move { server::serve(Some(&server_home), 0).await });
    let paths = storage::paths_for_home(Some(&home))?;
    let client = Client::builder().timeout(Duration::from_secs(1)).build()?;
    for _ in 0..100 {
        if let Ok(record) = lifecycle::read_record(&paths)
            && client
                .get(format!("http://{}/healthz", record.address))
                .send()
                .await
                .is_ok_and(|response| response.status().is_success())
        {
            let token = AuthState::read_bearer_file(&paths.runtime.join("bearer.token"))?;
            return Ok((task, paths, record, token));
        }
        if task.is_finished() {
            return match task.await? {
                Ok(()) => Err("local service exited before readiness".into()),
                Err(error) => Err(format!("local service startup failed: {error}").into()),
            };
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Err("local service did not become ready".into())
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn api_keeps_status_shape_and_enforces_local_session_contract()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;
    let base = format!("http://{}", record.address);

    let shell = client.get(format!("{base}/")).send().await?;
    let shell_html = shell.text().await?;
    let decoder_position = shell_html
        .find("/error-envelope.js")
        .ok_or("shell did not load the error-envelope decoder")?;
    let app_position = shell_html
        .find("/app.js")
        .ok_or("shell did not load the interface entrypoint")?;
    assert!(decoder_position < app_position);
    let inventory_script_position = shell_html
        .find("/connection-inventory.js")
        .ok_or("shell did not load the connection inventory renderer")?;
    assert!(inventory_script_position < app_position);
    let supply_comparison_script_position = shell_html
        .find("/market-supply-comparison.js")
        .ok_or("shell did not load the supply comparison renderer")?;
    assert!(supply_comparison_script_position < app_position);
    let decoder = client
        .get(format!("{base}/error-envelope.js"))
        .send()
        .await?;
    assert_eq!(decoder.status(), reqwest::StatusCode::OK);
    assert!(
        decoder
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/javascript;"))
    );
    assert_eq!(
        decoder.text().await?,
        include_str!("../../../apps/hyperai-interface/error-envelope.js")
    );

    let inventory_script = client
        .get(format!("{base}/connection-inventory.js"))
        .send()
        .await?;
    assert_eq!(inventory_script.status(), reqwest::StatusCode::OK);
    assert_eq!(
        inventory_script.text().await?,
        include_str!("../../../apps/hyperai-interface/connection-inventory.js")
    );

    let supply_comparison_script = client
        .get(format!("{base}/market-supply-comparison.js"))
        .send()
        .await?;
    assert_eq!(supply_comparison_script.status(), reqwest::StatusCode::OK);
    assert_eq!(
        supply_comparison_script.text().await?,
        include_str!("../../../apps/hyperai-interface/market-supply-comparison.js")
    );

    let unauthenticated_inventory = client
        .get(format!("{base}/api/v1/connections"))
        .send()
        .await?;
    assert_eq!(
        unauthenticated_inventory.status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    let unauthorized_body: Value = unauthenticated_inventory.json().await?;
    assert_eq!(unauthorized_body["code"], "UNAUTHENTICATED");

    let unauthenticated_comparison = client
        .get(format!("{base}/api/v1/market/supply-comparison"))
        .send()
        .await?;
    assert_eq!(
        unauthenticated_comparison.status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    let comparison = client
        .get(format!("{base}/api/v1/market/supply-comparison"))
        .bearer_auth(&bearer)
        .send()
        .await?;
    assert_eq!(comparison.status(), reqwest::StatusCode::OK);
    let comparison: Value = comparison.json().await?;
    assert_eq!(
        comparison["body"]["schema_version"],
        "hyperai.compute-market.supply-comparison.v1"
    );
    assert_eq!(comparison["body"]["selected_offer_id"], "offer-gpu-b");

    let supply_quote_route = format!("{base}/api/v1/market/supply-quote-preview");
    let unauthenticated_supply_quote = client
        .post(&supply_quote_route)
        .json(&serde_json::json!({ "comparison_digest": comparison["digest"] }))
        .send()
        .await?;
    assert_eq!(
        unauthenticated_supply_quote.status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    let supply_quote: Value = client
        .post(&supply_quote_route)
        .bearer_auth(&bearer)
        .json(&serde_json::json!({ "comparison_digest": comparison["digest"] }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        supply_quote["body"]["schema_version"],
        "hyperai.compute-market.supply-quote-preview.v1"
    );
    assert_eq!(
        supply_quote["body"]["comparison_digest"],
        comparison["digest"]
    );
    assert_eq!(supply_quote["body"]["selected_offer_id"], "offer-gpu-b");
    assert_eq!(
        supply_quote["body"]["quote_preview"]["body"]["selected_offer_id"],
        supply_quote["body"]["selected_offer_id"]
    );

    let stale_supply_quote = client
        .post(&supply_quote_route)
        .bearer_auth(&bearer)
        .json(&serde_json::json!({ "comparison_digest": "0".repeat(64) }))
        .send()
        .await?;
    assert_eq!(stale_supply_quote.status(), reqwest::StatusCode::CONFLICT);
    let stale_supply_quote: Value = stale_supply_quote.json().await?;
    assert_eq!(
        stale_supply_quote["code"],
        "MARKET_SUPPLY_COMPARISON_CHANGED"
    );

    let extra_supply_quote_field = client
        .post(&supply_quote_route)
        .bearer_auth(&bearer)
        .json(&serde_json::json!({
            "comparison_digest": comparison["digest"],
            "approve": true
        }))
        .send()
        .await?;
    assert_eq!(
        extra_supply_quote_field.status(),
        reqwest::StatusCode::BAD_REQUEST
    );
    let wrong_supply_quote_method = client
        .get(&supply_quote_route)
        .bearer_auth(&bearer)
        .send()
        .await?;
    assert_eq!(
        wrong_supply_quote_method.status(),
        reqwest::StatusCode::METHOD_NOT_ALLOWED
    );

    let inventory = client
        .get(format!("{base}/api/v1/connections"))
        .bearer_auth(&bearer)
        .send()
        .await?;
    assert_eq!(inventory.status(), reqwest::StatusCode::OK);
    assert!(inventory.headers().contains_key("x-request-id"));
    let inventory_body: Value = inventory.json().await?;
    assert_eq!(
        inventory_body["schema_version"],
        "hyperai.interface.connection-inventory.v1"
    );
    assert_eq!(inventory_body["mode"], "static_local_inventory");
    let inventory_connections = inventory_body["connections"]
        .as_array()
        .ok_or("connection inventory is not an array")?;
    assert_eq!(inventory_connections.len(), 11);
    assert_eq!(
        inventory_connections
            .iter()
            .map(|item| item["module_id"].as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        [
            "compute_market",
            "simple_compute_market",
            "compute_bazaar",
            "hyperintel",
            "fleet",
            "eval",
            "outcomes",
            "justai",
            "esn_justnet",
            "trade",
            "data",
        ]
    );
    let bazaar = &inventory_connections[2];
    assert_eq!(bazaar["connection_mode"], "reference_only");
    assert_eq!(bazaar["exposed_capability_ids"], serde_json::json!([]));
    assert_eq!(
        bazaar["next_blocked_gate"]["id"],
        "written_rights_for_exact_materials"
    );

    let status = client.get(format!("{base}/api/v1/status")).send().await?;
    assert_eq!(status.status(), reqwest::StatusCode::OK);
    assert!(status.headers().contains_key("x-request-id"));
    assert!(
        status
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none()
    );
    let status_body: Value = status.json().await?;
    assert_eq!(status_body["schema_version"], "hyperai.interface.status.v1");
    let status_object = status_body
        .as_object()
        .ok_or("status response is not an object")?;
    assert_eq!(status_object.len(), 3);
    assert!(status_object.contains_key("connections"));
    assert!(status_object.contains_key("mode"));
    assert!(status_object.contains_key("schema_version"));
    let connections = status_body["connections"]
        .as_array()
        .ok_or("status connections is not an array")?;
    let connection_ids = connections
        .iter()
        .map(|connection| connection["module_id"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(
        connection_ids,
        [
            "compute_market",
            "simple_compute_market",
            "compute_bazaar",
            "hyperintel",
            "fleet",
            "eval",
            "outcomes",
            "justai",
            "esn_justnet",
            "trade",
            "data",
        ]
    );
    assert_eq!(connections.len(), 11);
    assert!(connections.iter().all(|item| {
        item.as_object().is_some_and(|entry| {
            entry.len() == 5
                && ["boundary", "module_id", "name", "owner", "state"]
                    .iter()
                    .all(|key| entry.contains_key(*key))
        })
    }));
    let data_connection = connections
        .iter()
        .find(|connection| connection["module_id"] == "data")
        .ok_or("status inventory omitted the HyperAI Data workspace")?;
    assert_eq!(data_connection["name"], "HyperAI Data");
    assert_eq!(data_connection["state"], "local_synthetic_workspace");
    assert_eq!(data_connection["owner"], "HyperAI Data contracts");
    assert_eq!(
        data_connection["boundary"],
        "Manifest-verified local JSON/Parquet snapshots and bounded read-only queries; no external ingestion"
    );

    let invalid_request_id = client
        .get(format!("{base}/api/v1/status"))
        .header("x-request-id", "bad.id")
        .send()
        .await?;
    assert_eq!(
        invalid_request_id.status(),
        reqwest::StatusCode::BAD_REQUEST
    );
    assert!(invalid_request_id.headers().contains_key("x-request-id"));
    let invalid_id_body: Value = invalid_request_id.json().await?;
    assert_eq!(invalid_id_body["code"], "INVALID_REQUEST");

    let wrong_origin = client
        .get(format!("{base}/api/v1/status"))
        .header(header::ORIGIN, "http://attacker.invalid")
        .send()
        .await?;
    assert_eq!(wrong_origin.status(), reqwest::StatusCode::FORBIDDEN);

    let unauthenticated = client
        .post(format!("{base}/api/v1/market/demo"))
        .header(header::CONTENT_TYPE, "application/json")
        .body("{}")
        .send()
        .await?;
    assert_eq!(unauthenticated.status(), reqwest::StatusCode::UNAUTHORIZED);
    let unauthenticated_body: Value = unauthenticated.json().await?;
    assert_eq!(
        unauthenticated_body["schema_version"],
        "hyperai.interface.error.v1"
    );
    assert_eq!(unauthenticated_body["code"], "UNAUTHENTICATED");

    let missing_content_type = client
        .post(format!("{base}/api/v1/market/demo"))
        .body("{}")
        .send()
        .await?;
    assert_eq!(
        missing_content_type.status(),
        reqwest::StatusCode::BAD_REQUEST
    );
    let missing_type_body: Value = missing_content_type.json().await?;
    assert_eq!(missing_type_body["code"], "INVALID_JSON");

    let session_response = client.get(format!("{base}/api/v1/session")).send().await?;
    let cookie = session_response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("session did not set cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("session cookie was empty")?
        .to_owned();
    assert!(cookie.starts_with("hyperai_session="));
    assert!(
        session_response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("Max-Age=1800"))
    );
    assert_ne!(cookie.trim_start_matches("hyperai_session="), bearer);
    let session: Value = session_response.json().await?;
    assert_eq!(
        session["schema_version"],
        "hyperai.local-service.session.v1"
    );
    let csrf = session["csrf_token"].as_str().ok_or("missing CSRF value")?;

    let missing_csrf = client
        .post(format!("{base}/api/v1/market/demo"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ORIGIN, &base)
        .header(header::COOKIE, &cookie)
        .body("{}")
        .send()
        .await?;
    assert_eq!(missing_csrf.status(), reqwest::StatusCode::FORBIDDEN);

    let authorized = client
        .post(format!("{base}/api/v1/market/demo"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ORIGIN, &base)
        .header(header::COOKIE, &cookie)
        .header("x-csrf-token", csrf)
        .body("{}")
        .send()
        .await?;
    assert_eq!(authorized.status(), reqwest::StatusCode::OK);
    let demo: Value = authorized.json().await?;
    assert_eq!(demo["schema_version"], "hyperai.interface.market-demo.v1");

    let _rotated_session: Value = client
        .get(format!("{base}/api/v1/session"))
        .send()
        .await?
        .json()
        .await?;
    let stale_cookie = client
        .post(format!("{base}/api/v1/market/demo"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ORIGIN, &base)
        .header(header::COOKIE, &cookie)
        .header("x-csrf-token", csrf)
        .body("{}")
        .send()
        .await?;
    assert_eq!(stale_cookie.status(), reqwest::StatusCode::UNAUTHORIZED);

    let malformed = client
        .post(format!("{base}/api/v1/market/verify"))
        .bearer_auth(&bearer)
        .header(header::CONTENT_TYPE, "application/json")
        .body("{")
        .send()
        .await?;
    assert_eq!(malformed.status(), reqwest::StatusCode::BAD_REQUEST);
    let malformed_body: Value = malformed.json().await?;
    assert_eq!(malformed_body["code"], "INVALID_JSON");

    let too_large = client
        .post(format!("{base}/api/v1/market/demo"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(vec![b'x'; 1_048_577])
        .send()
        .await?;
    assert_eq!(too_large.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);
    let too_large_body: Value = too_large.json().await?;
    assert_eq!(too_large_body["code"], "BODY_TOO_LARGE");

    let wrong_host = client
        .get(format!("{base}/api/v1/status"))
        .header(header::HOST, "example.invalid")
        .send()
        .await?;
    assert_eq!(wrong_host.status(), reqwest::StatusCode::FORBIDDEN);

    let _ = client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn synthetic_market_history_is_owner_only_and_survives_service_restart()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("home");
    let (task, _, record, bearer) = launch(home.clone()).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);

    let anonymous = client
        .get(format!("{base}/api/v1/market/deal-history"))
        .send()
        .await?;
    assert_eq!(anonymous.status(), reqwest::StatusCode::UNAUTHORIZED);

    let preview: Value = client
        .post(format!("{base}/api/v1/market/quote-preview"))
        .bearer_auth(&bearer)
        .json(&serde_json::json!({}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let preview_digest = preview["digest"].clone();
    let completion_request = serde_json::json!({
        "preview_digest": preview_digest,
        "deal_digest": "00".repeat(32),
    });
    let unapproved = client
        .post(format!("{base}/api/v1/market/quote-complete"))
        .bearer_auth(&bearer)
        .json(&completion_request)
        .send()
        .await?;
    assert_eq!(unapproved.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(
        unapproved.json::<Value>().await?["code"],
        "MARKET_DEAL_NOT_APPROVED"
    );

    let approval: Value = client
        .post(format!("{base}/api/v1/market/quote-approve"))
        .bearer_auth(&bearer)
        .json(&serde_json::json!({ "preview_digest": preview_digest }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let completion_request = serde_json::json!({
        "preview_digest": preview_digest,
        "deal_digest": approval["deal_digest"],
    });

    let approved_page: LocalMarketHistoryPageV1 = client
        .get(format!("{base}/api/v1/market/deal-history"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    verify_local_market_history_page_v1(&approved_page)?;
    assert_eq!(approved_page.body.entries.len(), 1);
    assert_eq!(approved_page.body.entries[0].completion_receipt, None);

    let session_response = client.get(format!("{base}/api/v1/session")).send().await?;
    let cookie = session_response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("session did not set cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("session cookie was empty")?
        .to_owned();
    let session_page = client
        .get(format!("{base}/api/v1/market/deal-history"))
        .header(header::COOKIE, cookie)
        .send()
        .await?;
    assert_eq!(session_page.status(), reqwest::StatusCode::OK);

    let completion: Value = client
        .post(format!("{base}/api/v1/market/quote-complete"))
        .bearer_auth(&bearer)
        .json(&completion_request)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        completion["state_slice"],
        "hyperai.bazaar_interface.synthetic_quote_completion.v1"
    );
    let completed_page: LocalMarketHistoryPageV1 = client
        .get(format!("{base}/api/v1/market/deal-history?limit=1"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    verify_local_market_history_page_v1(&completed_page)?;
    assert!(completed_page.body.entries[0].completion_receipt.is_some());

    for invalid_query in ["?limit=0", "?cursor=not-a-cursor", "?limit=1&limit=2"] {
        let invalid = client
            .get(format!("{base}/api/v1/market/deal-history{invalid_query}"))
            .bearer_auth(&bearer)
            .send()
            .await?;
        assert_eq!(invalid.status(), reqwest::StatusCode::BAD_REQUEST);
        assert_eq!(
            invalid.json::<Value>().await?["code"],
            "MARKET_HISTORY_INVALID_QUERY"
        );
    }

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;

    let (task, _, restarted_record, restarted_bearer) = launch(home).await?;
    let restarted_base = format!("http://{}", restarted_record.address);
    let restarted_page: LocalMarketHistoryPageV1 = client
        .get(format!("{restarted_base}/api/v1/market/deal-history"))
        .bearer_auth(&restarted_bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    verify_local_market_history_page_v1(&restarted_page)?;
    assert_eq!(restarted_page, completed_page);
    client
        .post(format!("{restarted_base}/api/v1/service/stop"))
        .bearer_auth(&restarted_bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn synthetic_market_history_cursor_traverses_two_valid_deals_after_restart()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("home");
    let (task, _, record, bearer) = launch(home.clone()).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);

    let anonymous_alternate = client
        .post(format!(
            "{base}/api/v1/market/quote-preview/scenario/alternate"
        ))
        .json(&serde_json::json!({}))
        .send()
        .await?;
    assert_eq!(
        anonymous_alternate.status(),
        reqwest::StatusCode::UNAUTHORIZED
    );

    let invalid_scenario = client
        .post(format!(
            "{base}/api/v1/market/quote-preview/scenario/not-declared"
        ))
        .bearer_auth(&bearer)
        .json(&serde_json::json!({}))
        .send()
        .await?;
    assert_eq!(invalid_scenario.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(
        invalid_scenario.json::<Value>().await?["code"],
        "MARKET_QUOTE_SCENARIO_INVALID"
    );

    let baseline_preview: Value = client
        .post(format!("{base}/api/v1/market/quote-preview"))
        .bearer_auth(&bearer)
        .json(&serde_json::json!({}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let alternate_preview: Value = client
        .post(format!(
            "{base}/api/v1/market/quote-preview/scenario/alternate"
        ))
        .bearer_auth(&bearer)
        .json(&serde_json::json!({}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let baseline_preview_digest = baseline_preview["digest"]
        .as_str()
        .ok_or("missing baseline preview digest")?;
    let alternate_preview_digest = alternate_preview["digest"]
        .as_str()
        .ok_or("missing alternate preview digest")?;
    assert_ne!(baseline_preview_digest, alternate_preview_digest);
    assert_eq!(
        baseline_preview["body"]["buyer_request"]["buyer_id"],
        "synthetic-buyer"
    );
    assert_eq!(
        alternate_preview["body"]["buyer_request"]["buyer_id"],
        "synthetic-buyer-alternate"
    );

    let mut deal_digests = Vec::new();
    for preview_digest in [baseline_preview_digest, alternate_preview_digest] {
        let approval: Value = client
            .post(format!("{base}/api/v1/market/quote-approve"))
            .bearer_auth(&bearer)
            .json(&serde_json::json!({ "preview_digest": preview_digest }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let deal_digest = approval["deal_digest"]
            .as_str()
            .ok_or("missing approved deal digest")?;
        let completion = client
            .post(format!("{base}/api/v1/market/quote-complete"))
            .bearer_auth(&bearer)
            .json(&serde_json::json!({
                "preview_digest": preview_digest,
                "deal_digest": deal_digest,
            }))
            .send()
            .await?;
        assert_eq!(completion.status(), reqwest::StatusCode::OK);
        deal_digests.push(deal_digest.to_owned());
    }
    assert_ne!(deal_digests[0], deal_digests[1]);

    let first_page: LocalMarketHistoryPageV1 = client
        .get(format!("{base}/api/v1/market/deal-history?limit=1"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    verify_local_market_history_page_v1(&first_page)?;
    assert_eq!(first_page.body.entries.len(), 1);
    assert_eq!(
        first_page.body.entries[0].approval.deal_digest.to_string(),
        deal_digests[1]
    );
    assert!(first_page.body.entries[0].completion_receipt.is_some());
    let cursor = first_page
        .body
        .next_cursor
        .as_deref()
        .ok_or("first page did not return a continuation cursor")?;
    assert!(cursor.starts_with("mh1_"));

    let second_page: LocalMarketHistoryPageV1 = client
        .get(format!(
            "{base}/api/v1/market/deal-history?limit=1&cursor={cursor}"
        ))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    verify_local_market_history_page_v1(&second_page)?;
    assert_eq!(second_page.body.entries.len(), 1);
    assert_eq!(
        second_page.body.entries[0].approval.deal_digest.to_string(),
        deal_digests[0]
    );
    assert!(second_page.body.entries[0].completion_receipt.is_some());
    assert_eq!(second_page.body.next_cursor, None);
    assert_ne!(first_page.digest, second_page.digest);

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;

    let (task, _, restarted_record, restarted_bearer) = launch(home).await?;
    let restarted_base = format!("http://{}", restarted_record.address);
    let restarted_first_page: LocalMarketHistoryPageV1 = client
        .get(format!(
            "{restarted_base}/api/v1/market/deal-history?limit=1"
        ))
        .bearer_auth(&restarted_bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    verify_local_market_history_page_v1(&restarted_first_page)?;
    assert_eq!(restarted_first_page, first_page);

    let restarted_second_page: LocalMarketHistoryPageV1 = client
        .get(format!(
            "{restarted_base}/api/v1/market/deal-history?limit=1&cursor={cursor}"
        ))
        .bearer_auth(&restarted_bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    verify_local_market_history_page_v1(&restarted_second_page)?;
    assert_eq!(restarted_second_page, second_page);

    client
        .post(format!("{restarted_base}/api/v1/service/stop"))
        .bearer_auth(&restarted_bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}
