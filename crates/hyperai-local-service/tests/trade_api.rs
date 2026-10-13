//! Authenticated synthetic Trade catalog, scenario, and paper replay checks.

use std::{error::Error, path::PathBuf, time::Duration};

use hyperai_local_service::{
    AuthState,
    lifecycle::{self, ServiceRecordV1},
    server,
    storage::{self, HomePaths},
};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
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
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
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
async fn trade_routes_are_authenticated_bounded_deterministic_and_effect_free()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);

    let unauthorized = client
        .get(format!("{base}/api/v1/trade/catalog"))
        .send()
        .await?;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let catalog: Value = client
        .get(format!("{base}/api/v1/trade/catalog"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let repeated: Value = client
        .get(format!("{base}/api/v1/trade/catalog"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(catalog, repeated);
    assert_eq!(catalog["schema_version"], "hyperai.trade.catalog.v1");
    assert_eq!(
        catalog["dataset_id"],
        "hyperai.synthetic.compute-hourly-series.v1"
    );
    assert_eq!(catalog["freshness"], "synthetic_not_market_data");
    assert_eq!(catalog["instruments"].as_array().map(Vec::len), Some(3));
    assert!(
        catalog["instruments"]
            .as_array()
            .is_some_and(|items| items.iter().all(|item| item["tradable"] == false))
    );

    let series: Value = client
        .get(format!("{base}/api/v1/trade/series/a100-us-east"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(series["price_coverage_bps"], 8_750);
    assert_eq!(series["missing_periods"][0], "2026-01-01T04:00:00Z");
    let unknown_instrument = client
        .get(format!("{base}/api/v1/trade/series/unknown"))
        .bearer_auth(&bearer)
        .send()
        .await?;
    assert_eq!(unknown_instrument.status(), StatusCode::NOT_FOUND);

    let scenario_request = json!({
        "expected_dataset_digest": catalog["dataset_digest"],
        "primary_instrument_id": "a100-us-east",
        "comparison_instrument_id": "a100-us-west",
        "price_shock_bps": 1000,
        "availability_delta_units": 1
    });
    let scenario: Value = client
        .post(format!("{base}/api/v1/trade/scenario"))
        .bearer_auth(&bearer)
        .json(&scenario_request)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(scenario["regional_basis_usd_micros_per_hour"], -75_000);
    assert_eq!(
        scenario["scenario_primary_asking_usd_micros_per_hour"],
        1_402_500
    );
    assert_eq!(scenario["scenario_available_units"], 5);
    assert!(
        scenario["nonclaims"]
            .as_str()
            .is_some_and(|value| value.contains("No live market observation"))
    );

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn trade_paper_replay_binds_its_snapshot_and_enforces_request_limits()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);
    let catalog: Value = client
        .get(format!("{base}/api/v1/trade/catalog"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let paper: Value = client
        .post(format!("{base}/api/v1/trade/paper"))
        .bearer_auth(&bearer)
        .json(&json!({
            "expected_dataset_digest": catalog["dataset_digest"],
            "baseline_instrument_id": "a100-us-west",
            "requested_units": 1
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(paper["periods"].as_array().map(Vec::len), Some(8));
    assert_eq!(paper["unknown_baseline_capacity_count"], 1);
    assert_eq!(
        paper["measurement_type"],
        "synthetic_paper_procurement_asking_cost_replay_not_fill_or_pnl"
    );

    let changed_digest = client
        .post(format!("{base}/api/v1/trade/scenario"))
        .bearer_auth(&bearer)
        .json(&json!({
            "expected_dataset_digest": "0000000000000000000000000000000000000000000000000000000000000000",
            "primary_instrument_id": "a100-us-east",
            "comparison_instrument_id": "a100-us-west",
            "price_shock_bps": 0,
            "availability_delta_units": 0
        }))
        .send()
        .await?;
    assert_eq!(changed_digest.status(), StatusCode::CONFLICT);
    assert_eq!(
        changed_digest.json::<Value>().await?["code"],
        "TRADE_DATASET_CHANGED"
    );

    let oversized = client
        .post(format!("{base}/api/v1/trade/scenario"))
        .bearer_auth(&bearer)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(format!(r#"{{"padding":"{}"}}"#, "x".repeat(4 * 1024)))
        .send()
        .await?;
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        oversized.json::<Value>().await?["code"],
        "TRADE_REQUEST_TOO_LARGE"
    );

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn trade_browser_scenarios_require_the_session_csrf_token()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let (task, _, record, bearer) = launch(temporary.path().join("home")).await?;
    let client = Client::builder().timeout(Duration::from_secs(3)).build()?;
    let base = format!("http://{}", record.address);
    let catalog: Value = client
        .get(format!("{base}/api/v1/trade/catalog"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let session_response = client.get(format!("{base}/api/v1/session")).send().await?;
    let cookie = session_response
        .headers()
        .get("set-cookie")
        .ok_or("browser session did not set a cookie")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("browser session cookie was empty")?
        .to_owned();
    let session: Value = session_response.json().await?;
    let csrf = session["csrf_token"]
        .as_str()
        .ok_or("browser session did not return a CSRF token")?;
    let body = json!({
        "expected_dataset_digest": catalog["dataset_digest"],
        "primary_instrument_id": "a100-us-east",
        "comparison_instrument_id": "a100-us-west",
        "price_shock_bps": 0,
        "availability_delta_units": 0
    });
    let blocked = client
        .post(format!("{base}/api/v1/trade/scenario"))
        .header("origin", &base)
        .header("cookie", &cookie)
        .json(&body)
        .send()
        .await?;
    assert_eq!(blocked.status(), StatusCode::FORBIDDEN);
    let authorized = client
        .post(format!("{base}/api/v1/trade/scenario"))
        .header("origin", &base)
        .header("cookie", &cookie)
        .header("x-csrf-token", csrf)
        .json(&body)
        .send()
        .await?;
    assert_eq!(authorized.status(), StatusCode::OK);

    client
        .post(format!("{base}/api/v1/service/stop"))
        .bearer_auth(&bearer)
        .send()
        .await?
        .error_for_status()?;
    task.await??;
    Ok(())
}
