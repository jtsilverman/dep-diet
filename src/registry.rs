use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::Semaphore;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub unpacked_size: u64,
    pub file_count: u32,
    pub dep_count: u32,
}

#[derive(Deserialize)]
struct NpmResponse {
    name: Option<String>,
    version: Option<String>,
    dist: Option<NpmDist>,
    dependencies: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NpmDist {
    unpacked_size: Option<u64>,
    file_count: Option<u32>,
}

async fn fetch_package(client: &Client, name: &str) -> Option<PackageInfo> {
    let url = format!("https://registry.npmjs.org/{}/latest", name);
    let resp = client.get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let data: NpmResponse = resp.json().await.ok()?;
    let dist = data.dist?;

    Some(PackageInfo {
        name: data.name.unwrap_or_else(|| name.to_string()),
        version: data.version.unwrap_or_else(|| "?".to_string()),
        unpacked_size: dist.unpacked_size.unwrap_or(0),
        file_count: dist.file_count.unwrap_or(0),
        dep_count: data.dependencies.map(|d| d.len() as u32).unwrap_or(0),
    })
}

pub async fn fetch_all(names: &[String]) -> Vec<PackageInfo> {
    let client = Client::new();
    let semaphore = Arc::new(Semaphore::new(10));
    let mut handles = Vec::new();

    for name in names {
        let client = client.clone();
        let name = name.clone();
        let sem = semaphore.clone();

        handles.push(tokio::spawn(async move {
            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(_) => return None,
            };
            fetch_package(&client, &name).await
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Ok(Some(info)) = handle.await {
            results.push(info);
        }
    }

    results
}
