// Legacy auto preferences resolve locally to the default; never probe the network.
pub(crate) fn resolve(source: &str) -> Result<String, String> {
    let source = if source == "auto" { "direct" } else { source };
    crate::updates::route("", source)?;
    Ok(source.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_auto_and_manual_sources_resolve_without_network() {
        assert_eq!(resolve("auto").unwrap(), "direct");
        for source in ["direct", "proxy", "ghfast", "ghproxy_net"] {
            assert_eq!(resolve(source).unwrap(), source);
        }
        assert!(resolve("invalid").is_err());
    }
}
