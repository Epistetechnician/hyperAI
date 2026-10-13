//! Local process ownership, readiness, shutdown, and restart contract checks.

use std::{error::Error, path::PathBuf, time::Duration};

use hyperai_local_service::{
    AuthState,
    lifecycle::{self, LifecycleError, ServiceRecordV1},
    server,
    storage::{self, HomePaths},
};
use reqwest::Client;
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

async fn stop(
    client: &Client,
    record: &ServiceRecordV1,
    token: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let response = client
        .post(format!("http://{}/api/v1/service/stop", record.address))
        .bearer_auth(token)
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);
    Ok(())
}

#[tokio::test]
async fn service_lock_readiness_stop_and_restart_rotate_credentials()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary = tempfile::tempdir()?;
    let home = temporary.path().join("hyperai-home");
    let client = Client::builder().timeout(Duration::from_secs(2)).build()?;

    let (first_task, paths, first_record, first_token) = launch(home.clone()).await?;
    assert!(matches!(
        lifecycle::ServiceLock::acquire(&paths),
        Err(LifecycleError::AlreadyRunning)
    ));
    stop(&client, &first_record, &first_token).await?;
    first_task.await??;
    assert!(matches!(
        lifecycle::read_record(&paths),
        Err(LifecycleError::Io(ref error)) if error.kind() == std::io::ErrorKind::NotFound
    ));

    let (second_task, _, second_record, second_token) = launch(home).await?;
    assert_ne!(first_record.instance_id, second_record.instance_id);
    assert_ne!(first_token, second_token);
    stop(&client, &second_record, &second_token).await?;
    second_task.await??;
    Ok(())
}
