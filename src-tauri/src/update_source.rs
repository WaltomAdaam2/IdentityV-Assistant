use std::time::{Duration, Instant};
use tokio::sync::Mutex;

// Probe a fixed public installer so all routes transfer identical content.
const PROBE: &str = "https://github.com/WaltomAdaam2/desktop-releases/releases/download/v0.1.5/IdentityV-Assistant-0.1.5-Windows-x64-Setup.exe";
const SAMPLE_BYTES: usize = 256 * 1024;
static SELECTED: Mutex<Option<(Instant, String)>> = Mutex::const_new(None);

struct Measurement {
    source: &'static str,
    latency: f64,
    bytes_per_second: f64,
}

fn fastest(measurements: impl IntoIterator<Item = Measurement>) -> Result<String, String> {
    measurements
        .into_iter()
        .filter(|m| m.bytes_per_second.is_finite() && m.bytes_per_second > 0.0)
        // Compare predicted one-MiB transfer time, including response latency.
        .min_by(|a, b| {
            (a.latency + 1_048_576.0 / a.bytes_per_second)
                .total_cmp(&(b.latency + 1_048_576.0 / b.bytes_per_second))
        })
        .map(|m| m.source.to_string())
        .ok_or_else(|| "自动测速未找到可用线路，请选择更新源后重试".into())
}

async fn probe(client: &reqwest::Client, source: &'static str) -> Option<Measurement> {
    let started = Instant::now();
    let url = crate::updates::route(PROBE, source).ok()?;
    let mut response = client
        .get(url)
        .header("Range", "bytes=0-262143")
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?;
    let latency = started.elapsed().as_secs_f64();
    let transfer = Instant::now();
    let mut bytes = Vec::with_capacity(SAMPLE_BYTES);
    while bytes.len() < SAMPLE_BYTES {
        let chunk = response.chunk().await.ok()??;
        let remaining = SAMPLE_BYTES - bytes.len();
        bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
    }
    // Reject proxy error/login pages; normal download validation remains mandatory.
    if !bytes.starts_with(b"MZ") {
        return None;
    }
    Some(Measurement {
        source,
        latency,
        bytes_per_second: bytes.len() as f64 / transfer.elapsed().as_secs_f64().max(0.001),
    })
}

pub(crate) async fn resolve(source: &str) -> Result<String, String> {
    if source != "auto" {
        crate::updates::route("", source)?;
        return Ok(source.into());
    }
    let mut selected = SELECTED.lock().await;
    if let Some((checked, source)) = selected.as_ref() {
        if checked.elapsed() < Duration::from_secs(300) {
            return Ok(source.clone());
        }
    }
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .user_agent("IdentityV-Assistant")
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|error| error.to_string())?;
    let results = tokio::join!(
        probe(&client, "direct"),
        probe(&client, "proxy"),
        probe(&client, "ghfast"),
        probe(&client, "ghproxy_net")
    );
    let source = fastest(
        [results.0, results.1, results.2, results.3]
            .into_iter()
            .flatten(),
    )?;
    *selected = Some((Instant::now(), source.clone()));
    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_accounts_for_latency_and_throughput_and_rejects_no_results() {
        let choice = fastest([
            Measurement {
                source: "direct",
                latency: 0.01,
                bytes_per_second: 10_000.0,
            },
            Measurement {
                source: "proxy",
                latency: 0.2,
                bytes_per_second: 1_000_000.0,
            },
            Measurement {
                source: "ghfast",
                latency: 2.0,
                bytes_per_second: 2_000_000.0,
            },
        ])
        .unwrap();
        assert_eq!(choice, "proxy");
        assert!(fastest([]).is_err());
        assert!(fastest([Measurement {
            source: "direct",
            latency: 0.0,
            bytes_per_second: 0.0
        }])
        .is_err());
    }

    #[test]
    fn manual_selection_does_not_need_network() {
        tauri::async_runtime::block_on(async {
            assert_eq!(resolve("direct").await.unwrap(), "direct");
            assert_eq!(resolve("ghfast").await.unwrap(), "ghfast");
            assert!(resolve("invalid").await.is_err());
        });
    }

    #[test]
    #[ignore = "live public download-source measurements"]
    fn automatic_sources_live() {
        tauri::async_runtime::block_on(async {
            let source = resolve("auto").await.unwrap();
            assert!(["direct", "proxy", "ghfast", "ghproxy_net"].contains(&source.as_str()));
            assert_eq!(resolve("auto").await.unwrap(), source);
            println!("Selected source: {source}");
        });
    }
}
